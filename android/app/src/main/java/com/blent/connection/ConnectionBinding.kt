package com.blent.connection

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
    }
    private val projectionGate = Any()
    private val presentation = ConnectionPresentation()
    val state = presentation.state
    private val mutableCameraTarget = MutableStateFlow<CameraCaptureTarget?>(null)
    val cameraTarget = mutableCameraTarget.asStateFlow()
    private val lane = ResourceLane(scope, ::cleanup, cleanupNoticeMs)
    @Volatile private var current: Attempt? = null

    private fun cleanup(text: String) {
        if (lane.error != null) current?.let { presentation.ended(it.token, text) }
        presentation.cleanup(text)
    }

    @Synchronized fun connect(spec: ConnectionWorkerSpec) {
        try { lane.checkAvailable() }
        catch (failure: Exception) { presentation.cleanup(failure.message ?: "USB cleanup incomplete."); return }
        current?.cameras?.stop()
        val attempt = synchronized(projectionGate) {
            Attempt(presentation.begin()).also { current = it; mutableCameraTarget.value = null }
        }
        lane.replace({ open(attempt, spec) }, {
            attempt.cameras.stop()
            presentation.ended(attempt.token, it?.message)
            clearTarget(attempt)
            if (attempt.worker == null) presentation.retired(attempt.token)
        })
    }

    private fun receive(attempt: Attempt, event: ConnectionWorkerEvent) {
        presentation.event(attempt.token, event)
        if (event is ConnectionWorkerEvent.CameraClosed) attempt.cameras.closed(event.lease)
        if (event is ConnectionWorkerEvent.Ended) attempt.cameras.stop()
        projectCamera(attempt, event)
    }

    private fun clearTarget(attempt: Attempt) {
        synchronized(projectionGate) { if (current === attempt) mutableCameraTarget.value = null }
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
            override fun cancel() { attempt.cameras.stop(); worker.cancel() }
            override suspend fun retire() {
                worker.retire()
                attempt.cameras.retire()
                presentation.retired(attempt.token)
            }
        }
    }

    @Synchronized fun disconnect() {
        current?.let { it.cameras.stop(); presentation.stop(it.token) }
        mutableCameraTarget.value = null
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
        attempt.cameras.stop()
        mutableCameraTarget.value = null
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

    private fun send(attempt: Attempt, command: ConnectionWorkerCommand): Boolean = try {
        check(attempt.worker?.command(command) == true) { "USB command rejected; reconnect to try again." }
        true
    } catch (failure: Exception) {
        attempt.cameras.stop()
        clearTarget(attempt)
        presentation.ended(attempt.token, failure.message ?: "USB command failed.")
        lane.stop()
        false
    }
}
