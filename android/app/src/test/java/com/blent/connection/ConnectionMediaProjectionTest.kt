package com.blent.connection

import com.blent.*
import kotlinx.coroutines.*
import kotlinx.coroutines.flow.MutableStateFlow
import org.junit.Assert.*
import org.junit.Test

/** T990: metadata reuse never substitutes for original publication authority. */
class ConnectionMediaProjectionTest {
    private class Worker(val report: (ConnectionWorkerEvent) -> Unit) : ConnectionWorker {
        val release = CompletableDeferred<Unit>()
        override suspend fun execute() = awaitCancellation()
        override fun cancel() {}
        override suspend fun retire() = release.await()
        override fun command(value: ConnectionWorkerCommand) = true
    }
    private object NoNativeAuthority : MediaConsumerAuthority {
        override fun retain(): MediaConsumerClaim? = null
        override fun stop() = false
    }
    private class Rig : AutoCloseable {
        val scope = CoroutineScope(SupervisorJob() + Dispatchers.Unconfined)
        val workers = mutableListOf<Worker>()
        val binding = ConnectionBinding(scope, { _, report -> Worker(report).also { workers.add(it) } })
        val targets = MutableStateFlow<AudioConsumerTarget?>(null)
        var choice: ConnectionAudioChoice? = null
        var projections = 0
        val access = ConnectionMediaAccess<ConnectionAudioChoice, AudioAccessOffer, AudioEndpoint, AudioConsumerTarget>(
            binding, { choice }, targets,
            { selected ->
                projections++
                (selected as? ConnectionAudioRouteChoice)?.route?.let(ConnectionAudioTarget::endpoint)
            },
            { owner, selected, authority -> AudioAccessOffer(owner, 2, false, false,
                selected is ConnectionAudioRouteChoice, authority) },
            { _, _ -> true }, { _, _, _ -> true },
        )
        fun connect(): Worker {
            binding.connect(ConnectionWorkerSpec("test"))
            return workers.last().also {
                it.report(ConnectionWorkerEvent.Peer(audioLease().connection, ConnectionPeer.parse("cd".repeat(32)), false))
                it.report(ConnectionWorkerEvent.State(ConnectionWorkerPhase.AUTHENTICATED))
            }
        }
        fun select(route: LocalAudioRoute = audioRoute()) {
            choice = ConnectionAudioRouteChoice(route)
            targets.value = AudioConsumerTarget(ConnectionAudioTarget.endpoint(route), NoNativeAuthority)
        }
        override fun close() {
            workers.forEach { it.release.complete(Unit) }
            binding.disconnect(); scope.cancel()
        }
    }
    @Test fun t990UnchangedPublicationProjectsEndpointOnceAcrossSiblingAndTargetWakes() = Rig().use { rig ->
        val worker = rig.connect(); rig.select()
        val original = checkNotNull(rig.access.capture())
        repeat(64) { index ->
            worker.report(ConnectionWorkerEvent.Trust(audioLease().connection.peer, index % 2 == 0))
            rig.targets.value = AudioConsumerTarget(original.target.endpoint, NoNativeAuthority)
            val next = checkNotNull(rig.access.capture())
            assertSame(original.owner, next.owner)
            assertSame(rig.targets.value, next.target)
        }
        assertEquals("T990 endpoint projection belongs to exact publication", 1, rig.projections)
    }

    @Test fun t990ReplacementPublicationReprojectsEvenWhenEndpointValuesMatch() = Rig().use { rig ->
        rig.connect(); rig.select()
        val first = checkNotNull(rig.access.project())
        assertNotNull(rig.access.capture())
        rig.select()
        val next = checkNotNull(rig.access.project())
        assertSame(first.owner, next.owner)
        assertFalse(first.authority.current()); assertTrue(next.authority.current())
        assertNotNull(rig.access.capture())
        assertEquals("T990 exact choice replacements reproject", 2, rig.projections)
    }

    @Test fun t990CachedMetadataRejectsMismatchedTargetsAndWithdrawnOrForeignChoices() = Rig().use { rig ->
        rig.connect(); rig.select()
        val original = checkNotNull(rig.access.capture())
        rig.targets.value = AudioConsumerTarget(original.target.endpoint.copy(port = 12346), NoNativeAuthority)
        assertNull(rig.access.capture())
        rig.targets.value = null
        assertNull(rig.access.capture())
        rig.choice = null
        assertNull(rig.access.project()); assertNull(rig.access.capture())
        val route = audioRoute()
        rig.select(route.copy(lease = route.lease.copy(connection = route.lease.connection.copy(epoch = 2))))
        assertNull(rig.access.capture())
        rig.select(); assertNotNull(rig.access.capture())
        assertNotSame(original.owner, rig.access.capture()!!.owner)
    }

    @Test fun t990PeerReplacementClearsCacheWithoutRevivingOriginalAuthority() = Rig().use { rig ->
        val worker = rig.connect(); rig.select()
        val original = checkNotNull(rig.access.project())
        assertNotNull(rig.access.capture())
        rig.binding.disconnect()
        assertNull(rig.access.capture()); assertFalse(original.authority.current())
        worker.release.complete(Unit)
        rig.connect()
        val next = checkNotNull(rig.access.project())
        assertNotSame(original.owner, next.owner)
        assertTrue(next.authority.current()); assertFalse(original.authority.current())
        assertNotNull(rig.access.capture())
        assertEquals("T990 replacement peer reprojects equal choice", 2, rig.projections)
    }

    @Test fun t990BoundedJvmAllocationAndTimingMeasurement() = Rig().use { rig ->
        rig.connect(); rig.select()
        repeat(128) { checkNotNull(rig.access.capture()) }
        val allocations = JvmAllocations()
        repeat(3) { sample ->
            val before = allocations.read()
            val started = System.nanoTime()
            repeat(1000) { checkNotNull(rig.access.capture()) }
            val elapsed = System.nanoTime() - started
            val allocated = allocations.read() - before
            println("T990 sample=$sample captures=1000 bytes=$allocated nanos=$elapsed projections=${rig.projections}")
        }
    }
}
