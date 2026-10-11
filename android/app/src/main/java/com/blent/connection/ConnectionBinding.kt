package com.blent.connection

import com.blent.AudioConsumerTarget
import com.blent.MediaLocalConsent
import com.blent.CameraCaptureTarget
import com.blent.LifecycleRun
import com.blent.ResourceLane
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.asStateFlow

/** Process-owned UI controller. Native work belongs to the injected worker;
 * The factory must be lazy: no native work before execute.
 * ResourceLane retains ownership through execution AND actual retirement.
 * Public actions serialize independently from event/reducer callbacks, avoiding
 * lock inversion with the lane's factory, completion and deadline callbacks. */
internal class ConnectionBinding(
    scope: CoroutineScope,
    private val create: (ConnectionWorkerSpec, (ConnectionWorkerEvent) -> Unit) -> ConnectionWorker,
    cleanupNoticeMs: Long = 5000,
) {
    private class Attempt(val token: Any) {
        @Volatile var worker: ConnectionWorker? = null
        val cameras = ConnectionCameraSession({ worker?.command(it) == true })
        val microphones = ConnectionAudioSession(ConnectionManagement.Resource.MICROPHONE, { worker?.command(it) == true })
        val speakers = ConnectionAudioSession(ConnectionManagement.Resource.SPEAKERS, { worker?.command(it) == true })
        fun stopMedia() { cameras.stop(); microphones.stop(); speakers.stop() }
        suspend fun retireMedia() { cameras.retire(); microphones.retire(); speakers.retire() }
    }
    private val projectionGate = Any()
    private val presentation = ConnectionPresentation()
    val state = presentation.state
    private val mutableCameraTarget = MutableStateFlow<CameraCaptureTarget?>(null)
    val cameraTarget = mutableCameraTarget.asStateFlow()
    private val mutableMicrophoneTarget = MutableStateFlow<AudioConsumerTarget?>(null)
    val microphoneTarget = mutableMicrophoneTarget.asStateFlow()
    private val mutableSpeakersTarget = MutableStateFlow<AudioConsumerTarget?>(null)
    val speakersTarget = mutableSpeakersTarget.asStateFlow()
    private val lane = ResourceLane(scope, ::cleanup, cleanupNoticeMs)
    @Volatile private var current: Attempt? = null

    private fun cleanup(text: String) {
        if (lane.error != null) current?.let { presentation.ended(it.token, text) }
        presentation.cleanup(text)
    }

    @Synchronized fun connect(spec: ConnectionWorkerSpec) {
        try { lane.checkAvailable() }
        catch (failure: Exception) { presentation.cleanup(failure.message ?: "USB cleanup incomplete."); return }
        current?.stopMedia()
        val attempt = synchronized(projectionGate) {
            Attempt(presentation.begin()).also { current = it; clearTargets() }
        }
        lane.replace({ open(attempt, spec) }, {
            attempt.stopMedia()
            presentation.ended(attempt.token, it?.message)
            clearTarget(attempt)
            if (attempt.worker == null) presentation.retired(attempt.token)
        })
    }

    private fun receive(attempt: Attempt, event: ConnectionWorkerEvent) {
        presentation.event(attempt.token, event)
        if (event is ConnectionWorkerEvent.CameraClosed) attempt.cameras.closed(event.lease)
        if (event is ConnectionWorkerEvent.AudioClosed) {
            attempt.microphones.closed(event.lease); attempt.speakers.closed(event.lease)
        }
        if (event is ConnectionWorkerEvent.Ended) attempt.stopMedia()
        projectCamera(attempt, event)
        projectAudio(attempt, event)
    }

    private fun clearTargets() {
        mutableCameraTarget.value = null; mutableMicrophoneTarget.value = null; mutableSpeakersTarget.value = null
    }
    private fun projectAudio(attempt: Attempt, event: ConnectionWorkerEvent) = synchronized(projectionGate) {
        if (current !== attempt) return@synchronized
        projectAudioTarget(attempt.microphones, state.value.microphone?.route, event, mutableMicrophoneTarget)
        projectAudioTarget(attempt.speakers, state.value.speakers?.route, event, mutableSpeakersTarget)
    }
    private fun projectAudioTarget(session: ConnectionAudioSession, shown: LocalAudioRoute?, event: ConnectionWorkerEvent, target: MutableStateFlow<AudioConsumerTarget?>) {
        if (shown == null) { session.clear(); target.value = null }
        else if (event is ConnectionWorkerEvent.AudioRoute && shown === event.route) target.value = session.admitted(shown)
    }
    private fun clearTarget(attempt: Attempt) {
        synchronized(projectionGate) { if (current === attempt) clearTargets() }
    }

    private fun projectCamera(attempt: Attempt, event: ConnectionWorkerEvent) = synchronized(projectionGate) {
        if (current !== attempt) return@synchronized
        val shown = state.value.camera?.route
        if (shown == null) {
            attempt.cameras.clear()
            mutableCameraTarget.value = null
        } else if (event is ConnectionWorkerEvent.Camera && shown === event.route) {
            mutableCameraTarget.value = attempt.cameras.admitted(shown)
        }
    }

    private fun open(attempt: Attempt, spec: ConnectionWorkerSpec): LifecycleRun {
        val worker = create(spec) { receive(attempt, it) }
        attempt.worker = worker
        presentation.started(attempt.token)
        return object : LifecycleRun {
            override suspend fun execute() = worker.execute()
            override fun cancel() { attempt.stopMedia(); worker.cancel() }
            override suspend fun retire() {
                worker.retire()
                attempt.retireMedia()
                presentation.retired(attempt.token)
            }
        }
    }

    @Synchronized fun disconnect() {
        current?.let { it.stopMedia(); presentation.stop(it.token) }
        clearTargets()
        lane.stop()
    }

    @Synchronized fun decide(choice: ConnectionChoice, allow: Boolean, remember: Boolean): Boolean {
        val attempt = current ?: return false
        val command = presentation.decide(attempt.token, choice, allow, remember) ?: return false
        return send(attempt, command)
    }

    @Synchronized fun revoke(choice: ConnectionChoice): Boolean {
        val attempt = current ?: return false
        val command = presentation.revoke(attempt.token, choice) ?: return false
        attempt.stopMedia()
        clearTargets()
        return send(attempt, command)
    }

    fun cameraChoice(choice: ConnectionCameraChoice, context: CameraConsentChoice): Boolean {
        val attempt = current ?: return false
        val command = presentation.cameraChoice(attempt.token, choice, context) ?: return false
        return send(attempt, command)
    }

    fun cameraPermission(choice: ConnectionCameraOfferChoice, granted: Boolean, context: CameraConsentChoice): Boolean {
        val attempt = current ?: return false
        val command = presentation.cameraPermission(attempt.token, choice, granted, context) ?: return false
        return send(attempt, command)
    }

    fun audioChoice(choice: ConnectionAudioChoice, context: MediaLocalConsent): Boolean {
        val attempt = current ?: return false
        val command = presentation.audioChoice(attempt.token, choice, context) ?: return false
        return send(attempt, command)
    }
    fun audioPermission(choice: ConnectionAudioOfferChoice, granted: Boolean, context: MediaLocalConsent): Boolean {
        val attempt = current ?: return false
        val command = presentation.audioPermission(attempt.token, choice, granted, context) ?: return false
        return send(attempt, command)
    }

    private fun send(attempt: Attempt, command: ConnectionWorkerCommand): Boolean = try {
        check(attempt.worker?.command(command) == true) { "USB command rejected; reconnect to try again." }
        true
    } catch (failure: Exception) {
        attempt.stopMedia()
        clearTarget(attempt)
        presentation.ended(attempt.token, failure.message ?: "USB command failed.")
        lane.stop()
        false
    }
}
