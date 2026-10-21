package com.blent

import com.blent.connection.*
import java.util.concurrent.CopyOnWriteArrayList
import java.util.concurrent.atomic.AtomicInteger
import kotlinx.coroutines.*
import kotlinx.coroutines.flow.MutableStateFlow

/** T1242: production consent/consumer composition; explicit synthetic transport
 * and native completion gates. No Android device or process-death evidence. */
internal class MediaControlsReconnectFixture : AutoCloseable {
    class Transport(val report: (ConnectionWorkerEvent) -> Unit) : ConnectionWorker {
        val released = CompletableDeferred<Unit>()
        val commands = CopyOnWriteArrayList<ConnectionWorkerCommand>()
        override suspend fun execute() = awaitCancellation()
        override fun cancel() {}
        override suspend fun retire() = released.await()
        override fun command(value: ConnectionWorkerCommand) = commands.add(value)
    }

    class NativeRun(val report: (String) -> Unit) : LifecycleRun {
        val released = CompletableDeferred<Unit>()
        @Volatile var started = false
        val cancellations = AtomicInteger()
        @Volatile var retired = false
        override suspend fun execute() { started = true; awaitCancellation() }
        override fun cancel() { cancellations.incrementAndGet() }
        override suspend fun retire() { released.await(); retired = true }
    }

    val scope = CoroutineScope(SupervisorJob() + Dispatchers.Unconfined)
    val transports = CopyOnWriteArrayList<Transport>()
    val connection = ConnectionBinding(scope, { _, report -> Transport(report).also(transports::add) })
    val runs = List(3) { CopyOnWriteArrayList<NativeRun>() }
    var microphonePermission = true
    val camera = CameraBinding({ 0 }, { error("T1242 unexpected legacy permission") }, { true },
        { _, _, _, report -> native(0, report) }, MutableStateFlow(null), scope, {})
    val microphone = audio(1)
    val speakers = audio(2)
    val prompts = LocalPermissionPrompts()
    val requested = CopyOnWriteArrayList<LocalPermissionPrompt>()
    val cameraPolicy = MediaConsentPolicy<CameraAccessOffer>({ true }, prompts)
    private val cameraAccess = ConnectionCameraAccess(connection)
    private val cameraConsent = MediaConsentObservation(cameraAccess.offers, cameraPolicy, scope)
    private val cameraCapture = MediaConsumerObservation(cameraAccess.captures, cameraPolicy.state,
        camera::hasPermission, camera::useTarget, scope, ::CameraCaptureTarget)
    private val microphoneObservation = AudioAccessObservation(microphone,
        ConnectionAudioAccess(connection, ConnectionManagement.Resource.MICROPHONE), prompts, scope)
    private val speakerObservation = AudioAccessObservation(speakers,
        ConnectionAudioAccess(connection, ConnectionManagement.Resource.SPEAKERS), prompts, scope)
    val microphonePolicy get() = microphoneObservation.policy
    val speakerPolicy get() = speakerObservation.policy

    init {
        prompts.attach { requested.add(it); true }
        camera.start(); microphone.start(); speakers.start()
        cameraPolicy.foreground(true); microphonePolicy.foreground(true); speakerPolicy.foreground(true)
    }

    private fun native(index: Int, report: (String) -> Unit) = NativeRun(report).also(runs[index]::add)

    private fun audio(direction: Int) = AudioBinding(AudioPreferences(),
        { error("T1242 unexpected legacy permission") }, { direction == 2 || microphonePermission },
        { _, _, report -> native(direction, report) }, MutableStateFlow(null), scope, {}, direction, {})

    fun connect(): Transport {
        connection.connect(ConnectionWorkerSpec("T1242 synthetic transport"))
        return transports.last().also(::offer)
    }

    fun offer(worker: Transport) {
        worker.report(ConnectionWorkerEvent.Peer(cameraLease().connection, ConnectionPeer.parse("cd".repeat(32)), true))
        worker.report(ConnectionWorkerEvent.State(ConnectionWorkerPhase.AUTHENTICATED))
        worker.report(ConnectionWorkerEvent.CameraOffered(LocalCameraOffer(cameraLease(), cameraProfile(), false)))
        audioOffer(worker, ConnectionManagement.Resource.MICROPHONE)
        audioOffer(worker, ConnectionManagement.Resource.SPEAKERS)
    }

    private fun audioOffer(worker: Transport, resource: ConnectionManagement.Resource) {
        val route = audioRoute(resource)
        worker.report(ConnectionWorkerEvent.AudioOffered(LocalAudioOffer(route.lease, route.profile, false)))
    }

    fun admit(worker: Transport) {
        worker.report(ConnectionWorkerEvent.Camera(cameraRoute()))
        worker.report(ConnectionWorkerEvent.AudioRoute(audioRoute(ConnectionManagement.Resource.MICROPHONE)))
        worker.report(ConnectionWorkerEvent.AudioRoute(audioRoute(ConnectionManagement.Resource.SPEAKERS)))
    }

    fun closed(worker: Transport, index: Int) {
        val events = listOf(ConnectionWorkerEvent.CameraClosed(cameraLease()),
            ConnectionWorkerEvent.AudioClosed(audioLease(ConnectionManagement.Resource.MICROPHONE)),
            ConnectionWorkerEvent.AudioClosed(audioLease(ConnectionManagement.Resource.SPEAKERS)))
        worker.report(events[index])
    }

    fun terminalCommands(worker: Transport) = worker.commands.filter {
        it is ConnectionWorkerCommand.CameraConsumerRetired || it is ConnectionWorkerCommand.AudioConsumerRetired
    }

    override fun close() {
        connection.disconnect()
        cameraCapture.retire(); cameraConsent.retire()
        microphoneObservation.retire(); speakerObservation.retire()
        cameraPolicy.clear(); microphonePolicy.clear(); speakerPolicy.clear(); prompts.clear()
        camera.shutdown(); microphone.shutdown(); speakers.shutdown()
        runs.flatten().forEach { it.released.complete(Unit) }
        transports.forEach { it.released.complete(Unit) }
        scope.cancel()
        runBlocking { scope.coroutineContext[Job]!!.join() }
        check(runs.flatten().all { it.retired }) { "T1242 fixture leaked an original consumer" }
    }
}
