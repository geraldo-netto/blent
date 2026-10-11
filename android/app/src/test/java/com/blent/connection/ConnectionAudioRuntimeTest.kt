package com.blent.connection

import com.blent.MediaLocalConsent
import kotlinx.coroutines.*
import org.junit.Assert.*
import org.junit.Test

internal class AudioRuntimeRun : ConnectionRun, LocalAudioControl {
    val endpoints = ResourceEndpoints()
    val offers = mutableMapOf<ConnectionManagement.Resource, LocalAudioOffer>()
    val routes = mutableMapOf<ConnectionManagement.Resource, LocalAudioRoute>()
    val choices = mutableListOf<ConnectionWorkerCommand.AudioChoice>()
    val proofs = mutableListOf<ConnectionResourceLease>()
    var stopped = false
    var gone = false
    override fun display(): LocalDisplayRoute? = null
    override fun endpoints(): ConnectionEndpoints = endpoints
    override fun advance(now: Long) {}
    override fun stop(now: Long) { stopped = true }
    override fun failure(): Exception? = null
    override fun retired() = gone
    override fun terminationRequired() = false
    override fun audioOffered(resource: ConnectionManagement.Resource) = offers[resource]
    override fun audio(resource: ConnectionManagement.Resource) = routes[resource]
    override fun chooseAudio(lease: ConnectionResourceLease, choice: MediaLocalConsent, now: Long): Boolean {
        if (offers[lease.key.resource]?.lease != lease && routes[lease.key.resource]?.lease != lease) return false
        choices.add(ConnectionWorkerCommand.AudioChoice(lease, choice)); return true
    }
    override fun audioConsumerRetired(lease: ConnectionResourceLease, now: Long): Boolean {
        if (routes[lease.key.resource]?.lease != lease || lease in proofs) return false
        proofs.add(lease)
        if (proofs.size == routes.size) gone = true
        return true
    }
}

class ConnectionAudioRuntimeTest {
    private val resources = listOf(ConnectionManagement.Resource.MICROPHONE, ConnectionManagement.Resource.SPEAKERS)
    @Test fun t989RuntimeForwardsOnlyOriginalCurrentAudioControlsAndPreservesLateCompletion() {
        val setup = RuntimeSetup().apply { offer() }; val run = AudioRuntimeRun()
        val runtime = ConnectionRuntimeOwner(setup, ConnectionRunFactory { run }, { 0 })
        for (resource in resources) {
            val route = audioRoute(resource)
            run.offers[resource] = LocalAudioOffer(route.lease, route.profile)
            assertNull(runtime.audioOffered(resource)); assertNull(runtime.audio(resource))
        }
        runtime.advance()
        for (resource in resources) {
            val route = audioRoute(resource); assertEquals(run.offers[resource], runtime.audioOffered(resource))
            assertTrue(runtime.command(ConnectionWorkerCommand.AudioChoice(route.lease, MediaLocalConsent(true, true, true, false))))
            run.offers.remove(resource); run.routes[resource] = route
            assertNull(runtime.audio(resource)); run.endpoints.active.add(route.lease)
            assertSame(route, runtime.audio(resource)); assertNull(runtime.audioOffered(resource))
            val foreign = route.lease.copy(connection = route.lease.connection.copy(epoch = 2))
            assertFalse(runtime.command(ConnectionWorkerCommand.AudioChoice(foreign, MediaLocalConsent(true, true, true, false))))
        }
        assertEquals(2, run.choices.size)
        setup.live = false; runtime.advance()
        for (resource in resources) {
            assertNull(runtime.audioOffered(resource)); assertNull(runtime.audio(resource))
            assertFalse(runtime.command(ConnectionWorkerCommand.AudioChoice(audioLease(resource), MediaLocalConsent(true, true, true, false))))
            assertTrue(runtime.command(ConnectionWorkerCommand.AudioConsumerRetired(audioLease(resource))))
            assertFalse(runtime.command(ConnectionWorkerCommand.AudioConsumerRetired(audioLease(resource))))
        }
        assertTrue(run.stopped)
        val unsupported = RuntimeRig(); unsupported.start()
        assertNull(unsupported.owner.audioOffered(resources.first())); assertNull(unsupported.owner.audio(resources.first()))
        assertFalse(unsupported.owner.command(ConnectionWorkerCommand.AudioConsumerRetired(audioLease())))
    }
    @Test fun t989ActorPublishesEachDirectionOnceAndRetainsOriginalCompletionAfterCancel() = runBlocking {
        val setup = RuntimeSetup().apply { offer() }; val run = AudioRuntimeRun()
        val events = java.util.concurrent.CopyOnWriteArrayList<ConnectionWorkerEvent>()
        for (resource in resources) {
            val route = audioRoute(resource); run.routes[resource] = route; run.endpoints.active.add(route.lease)
        }
        val worker = AndroidConnectionWorker(ConnectionWorkerSpec("usb", timerMs = 1), { _, _ -> object : ConnectionSetup by setup {
            override fun stop() { setup.stop(); setup.gone = true }
        } }, events::add, { 0 }, ConnectionRunFactory { run })
        val execution = async { worker.execute() }
        try {
            until { events.filterIsInstance<ConnectionWorkerEvent.AudioRoute>().size == 2 }
            delay(5); assertEquals(2, events.size)
            assertTrue(worker.command(ConnectionWorkerCommand.AudioChoice(audioLease(), MediaLocalConsent(true, true, true, false))))
            until { run.choices.size == 1 }; worker.cancel()
            until { events.filterIsInstance<ConnectionWorkerEvent.AudioClosed>().size == 2 }
            assertFalse(worker.command(ConnectionWorkerCommand.AudioChoice(audioLease(), MediaLocalConsent(true, true, true, false))))
            assertTrue(worker.command(ConnectionWorkerCommand.AudioConsumerRetired(audioLease(resources.first()))))
            until { run.proofs.size == 1 }; assertFalse(execution.isCompleted)
            assertTrue(worker.command(ConnectionWorkerCommand.AudioConsumerRetired(audioLease(resources.last()))))
            execution.await(); worker.retire()
            assertFalse(worker.command(ConnectionWorkerCommand.AudioConsumerRetired(audioLease())))
            assertEquals(2, run.proofs.size)
        } finally { run.gone = true; setup.gone = true; worker.cancel(); execution.cancel(); worker.retire() }
    }
    private suspend fun until(condition: () -> Boolean) { withTimeout(2000) { while (!condition()) delay(1) } }
    @Test fun t989RuntimeRejectsWrongDirectionSnapshotsBeforePublishingOriginalTarget() {
        val setup = RuntimeSetup().apply { offer() }; val run = AudioRuntimeRun()
        val mic = audioRoute(resources.first()); run.endpoints.active.add(mic.lease)
        run.routes[resources.last()] = mic
        run.offers[resources.last()] = LocalAudioOffer(mic.lease, mic.profile)
        val runtime = ConnectionRuntimeOwner(setup, ConnectionRunFactory { run }, { 0 }); runtime.advance()
        assertNull("T989 requested speaker cannot publish microphone route", runtime.audio(resources.last()))
        assertNull("T989 requested speaker cannot publish microphone offer", runtime.audioOffered(resources.last()))
    }
}
