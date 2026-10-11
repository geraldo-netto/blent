package com.blent.connection

import com.blent.MediaLocalConsent
import com.blent.ResourceRetirement
import java.util.concurrent.CopyOnWriteArrayList
import kotlinx.coroutines.*
import org.junit.Assert.*
import org.junit.Test

class ConnectionAudioBindingTest {
    private val spec = ConnectionWorkerSpec("test")
    private val resources = listOf(ConnectionManagement.Resource.MICROPHONE, ConnectionManagement.Resource.SPEAKERS)
    private class Worker(val report: (ConnectionWorkerEvent) -> Unit) : ConnectionWorker {
        val released = CompletableDeferred<Unit>()
        val commands = CopyOnWriteArrayList<ConnectionWorkerCommand>()
        override suspend fun execute() = awaitCancellation()
        override fun cancel() {}
        override suspend fun retire() = released.await()
        override fun command(value: ConnectionWorkerCommand): Boolean = commands.add(value)
    }
    private inner class Rig : AutoCloseable {
        val scope = CoroutineScope(SupervisorJob() + Dispatchers.Unconfined)
        val workers = mutableListOf<Worker>()
        val binding = ConnectionBinding(scope, { _, report -> Worker(report).also { workers.add(it) } })
        fun connect(): Worker { binding.connect(spec); return workers.last().also { authenticate(it) } }
        fun authenticate(worker: Worker) {
            worker.report(ConnectionWorkerEvent.Peer(audioLease().connection, ConnectionPeer.parse("cd".repeat(32)), false))
            worker.report(ConnectionWorkerEvent.State(ConnectionWorkerPhase.AUTHENTICATED))
        }
        fun admit(worker: Worker, resource: ConnectionManagement.Resource) {
            val route = audioRoute(resource)
            worker.report(ConnectionWorkerEvent.AudioOffered(LocalAudioOffer(route.lease, route.profile)))
            worker.report(ConnectionWorkerEvent.AudioRoute(route))
        }
        override fun close() { workers.forEach { it.released.complete(Unit) }; binding.disconnect(); scope.cancel() }
    }
    private suspend fun until(done: () -> Boolean) = withTimeout(2000) { while (!done()) delay(1) }
    @Test fun t989AudioDirectionsRemainIndependentAndLateClaimsRelayOnlyToOriginalWorker() = runBlocking {
        Rig().use { rig ->
            val first = rig.connect(); resources.forEach { rig.admit(first, it) }
            val mic = checkNotNull(rig.binding.microphoneTarget.value); val speaker = checkNotNull(rig.binding.speakersTarget.value)
            val micClaim = checkNotNull(mic.authority.retain()); val speakerClaim = checkNotNull(speaker.authority.retain())
            first.report(ConnectionWorkerEvent.Display(displayRoute())); val display = rig.binding.state.value.display
            first.report(ConnectionWorkerEvent.CameraOffered(LocalCameraOffer(cameraLease(), cameraProfile(), false)))
            first.report(ConnectionWorkerEvent.Camera(cameraRoute())); val camera = rig.binding.cameraTarget.value
            first.report(ConnectionWorkerEvent.AudioClosed(audioLease(resources.first())))
            assertNull(rig.binding.microphoneTarget.value); assertSame(speaker, rig.binding.speakersTarget.value)
            assertSame(display, rig.binding.state.value.display); assertSame(camera, rig.binding.cameraTarget.value)
            assertFalse(micClaim.current()); assertTrue(speakerClaim.current()); assertTrue(first.commands.isEmpty())
            micClaim.retired(ResourceRetirement.RETIRED); until { first.commands.size == 1 }
            assertEquals(ConnectionWorkerCommand.AudioConsumerRetired(audioLease(resources.first())), first.commands.single())
            rig.binding.connect(spec); assertFalse(speakerClaim.current()); assertNull(rig.binding.speakersTarget.value)
            assertEquals(1, rig.workers.size)
            first.report(ConnectionWorkerEvent.AudioClosed(audioLease(resources.last())))
            speakerClaim.retired(ResourceRetirement.NOT_STARTED); until { first.commands.size == 2 }
            first.released.complete(Unit); until { rig.workers.size == 2 }
            val next = rig.workers.last(); rig.authenticate(next); resources.forEach { rig.admit(next, it) }
            val nextSpeaker = checkNotNull(rig.binding.speakersTarget.value)
            assertNotSame(speaker, nextSpeaker)
            first.report(ConnectionWorkerEvent.AudioClosed(audioLease(resources.last())))
            first.report(ConnectionWorkerEvent.AudioRoute(audioRoute())); speakerClaim.retired(ResourceRetirement.RETIRED)
            assertSame(nextSpeaker, rig.binding.speakersTarget.value); assertTrue(next.commands.isEmpty())
        }
    }
    @Test fun t989OnlyExactAuthenticatedAudioPublicationCanSubmitLocalChoice() {
        Rig().use { rig ->
            val forged = ConnectionAudioOfferChoice(LocalAudioOffer(audioLease(), audioRoute().profile))
            val context = MediaLocalConsent(true, true, true, false)
            assertFalse(rig.binding.audioChoice(forged, context))
            val worker = rig.connect(); worker.report(ConnectionWorkerEvent.AudioOffered(forged.offer))
            val offer = checkNotNull(rig.binding.state.value.speakersOffer)
            assertFalse(rig.binding.audioChoice(forged, context)); assertTrue(rig.binding.audioChoice(offer, context))
            assertEquals(ConnectionWorkerCommand.AudioChoice(audioLease(), context), worker.commands.single())
            worker.report(ConnectionWorkerEvent.AudioRoute(audioRoute()))
            assertFalse(rig.binding.audioChoice(offer, context))
            val admitted = checkNotNull(rig.binding.state.value.speakers)
            assertTrue(rig.binding.audioChoice(admitted, context))
            rig.binding.disconnect(); assertFalse(rig.binding.audioChoice(admitted, context))
        }
    }
    @Test fun t989ForeignOrUnauthenticatedAudioCannotCreateTargetsAndStopRevokesEveryDirection() {
        Rig().use { rig ->
            rig.binding.connect(spec); val worker = rig.workers.single(); resources.forEach { rig.admit(worker, it) }
            assertNull(rig.binding.microphoneTarget.value); assertNull(rig.binding.speakersTarget.value)
            rig.authenticate(worker)
            worker.report(ConnectionWorkerEvent.AudioRoute(audioRoute().copy(lease = audioLease().copy(connection = audioLease().connection.copy(epoch = 2)))))
            assertNull(rig.binding.speakersTarget.value)
            resources.forEach { rig.admit(worker, it) }
            val mic = checkNotNull(rig.binding.microphoneTarget.value); val speaker = checkNotNull(rig.binding.speakersTarget.value)
            worker.report(ConnectionWorkerEvent.State(ConnectionWorkerPhase.CLOSING))
            assertNull(rig.binding.microphoneTarget.value); assertNull(rig.binding.speakersTarget.value)
            assertNull(mic.authority.retain()); assertNull(speaker.authority.retain())
            worker.report(ConnectionWorkerEvent.AudioRoute(audioRoute())); assertNull(rig.binding.speakersTarget.value)
        }
    }
}
