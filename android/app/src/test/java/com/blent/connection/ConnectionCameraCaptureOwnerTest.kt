package com.blent.connection

import com.blent.*
import kotlinx.coroutines.*
import org.junit.Assert.*
import org.junit.Test
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicInteger
import java.util.Random

class ConnectionCameraCaptureOwnerTest {
    private val peer = ConnectionPeer.parse("ab".repeat(32))
    private val lease = ConnectionResourceLease(ConnectionTicket(1, peer),
        ConnectionManagement.Key(ConnectionManagement.Resource.CAMERA, 1, 1), 1)
    private val profile = ConnectionManagement.Camera(640, 480, 30, 3000, 1000, 150, 1, true, true)
    private fun route() = LocalCameraRoute(lease, ConnectionManagement.Credential(ByteArray(32) { it.toByte() }), 12345, profile)

    @Test fun t968DefaultIoAdapterRetainsOriginalClaimUntilActualCompletion() = runBlocking {
        val commands = java.util.concurrent.CopyOnWriteArrayList<ConnectionWorkerCommand>()
        val completed = CompletableDeferred<ConnectionWorkerCommand>()
        val owner = ConnectionMediaConsumerOwner(route().lease, ConnectionCameraConsumerCommands, { command ->
            commands.add(command)
            if (command is ConnectionWorkerCommand.CameraConsumerRetired) completed.complete(command)
            true
        })
        try {
            val claim = checkNotNull(owner.retain())
            assertFalse(owner.closed(lease.copy(owner = 2)))
            assertTrue(claim.current())
            assertTrue(owner.stop())
            assertFalse(claim.current())
            assertNull(owner.retain())
            assertTrue(owner.closed(lease))
            assertFalse("T968 route closure cannot replace native retirement", completed.isCompleted)
            assertEquals(listOf(ConnectionWorkerCommand.CameraChoice(lease,
                CameraConsentChoice(false, false, enabled = false))), commands)
            claim.retired(ResourceRetirement.RETIRED)
            assertEquals(ConnectionWorkerCommand.CameraConsumerRetired(lease),
                withTimeout(2000) { completed.await() })
            owner.retire()
            claim.retired(ResourceRetirement.RETIRED)
            assertFalse(owner.closed(lease))
            assertFalse(owner.stop())
            assertEquals(2, commands.size)
        } finally { owner.retire() }
    }

    @Test fun t948ClosureJoinsAllOriginalClaimsInEitherOrderWithoutDuplicateProof() = runBlocking {
        for (closeFirst in listOf(false, true)) {
            val commands = mutableListOf<ConnectionWorkerCommand>()
            val owner = ConnectionMediaConsumerOwner(route().lease, ConnectionCameraConsumerCommands, { commands.add(it); true }, Dispatchers.Unconfined)
            val first = checkNotNull(owner.retain())
            val second = checkNotNull(owner.retain())
            try {
                assertTrue(first.current())
                assertFalse(owner.closed(lease.copy(owner = 2)))
                if (closeFirst) assertTrue(owner.closed(lease))
                first.retired(ResourceRetirement.RETIRED)
                assertTrue(commands.isEmpty())
                second.retired(ResourceRetirement.NOT_STARTED)
                if (!closeFirst) { assertTrue(commands.isEmpty()); assertTrue(owner.closed(lease)) }
                assertEquals(listOf(ConnectionWorkerCommand.CameraConsumerRetired(lease)), commands)
                assertFalse(first.current())
                assertNull(owner.retain())
                first.retired(ResourceRetirement.RETIRED); owner.closed(lease)
                assertEquals(1, commands.size)
            } finally { owner.retire() }
        }
    }

    @Test fun t948LocalStopRevokesNewClaimsWithoutPretendingRouteClosure() = runBlocking {
        val commands = mutableListOf<ConnectionWorkerCommand>()
        val owner = ConnectionMediaConsumerOwner(route().lease, ConnectionCameraConsumerCommands, { commands.add(it); true }, Dispatchers.Unconfined)
        try {
            val claim = checkNotNull(owner.retain())
            assertTrue(owner.stop())
            assertFalse(claim.current())
            assertNull(owner.retain())
            claim.retired(ResourceRetirement.RETIRED)
            assertEquals(listOf(ConnectionWorkerCommand.CameraChoice(lease,
                CameraConsentChoice(false, false, enabled = false))), commands)
            owner.closed(lease)
            assertEquals(ConnectionWorkerCommand.CameraConsumerRetired(lease), commands.last())
        } finally { owner.retire() }
    }

    @Test fun t948RelaySenderCannotHoldDomainGateAcrossIndependentCallbacks() = runBlocking {
        val callbackFinished = CountDownLatch(1)
        var progressed = false
        lateinit var relay: ConnectionMediaCompletionRelay
        relay = ConnectionMediaCompletionRelay(ConnectionCameraConsumerCommands, lease, {
            val callback = Thread { relay.ready(); callbackFinished.countDown() }
            callback.start()
            progressed = callbackFinished.await(500, TimeUnit.MILLISECONDS)
            progressed
        }, Dispatchers.Unconfined)
        try { relay.ready(); assertTrue("T948 sender must not hold domain gate", progressed) }
        finally { relay.retire() }
    }

    @Test fun t948ActualRetirementRejectsFurtherStopAndMalformedLeasesRejectBeforeRelay() = runBlocking {
        val commands = mutableListOf<ConnectionWorkerCommand>()
        val owner = ConnectionMediaConsumerOwner(route().lease, ConnectionCameraConsumerCommands, { commands.add(it); true }, Dispatchers.Unconfined)
        owner.retire()
        assertFalse(owner.stop())
        assertTrue(commands.isEmpty())
        assertFalse(owner.closed(lease))
        assertNull(owner.retain())
    }

    @Test fun t948MalformedLeaseAndRetryParametersRejectBeforeAnyRelay() {
        val invalid = listOf(lease.copy(owner = 0), lease.copy(connection = ConnectionTicket(0, peer)),
            lease.copy(key = ConnectionManagement.Key(ConnectionManagement.Resource.DISPLAY, 1, 1)))
        for (value in invalid) assertThrows(IllegalArgumentException::class.java) {
            ConnectionMediaCompletionRelay(ConnectionCameraConsumerCommands, value, { error("invalid lease sent") }, Dispatchers.Unconfined)
        }
        for (value in listOf(Long.MIN_VALUE, -1, 0, 1001, Long.MAX_VALUE)) {
            assertThrows(IllegalArgumentException::class.java) {
                ConnectionMediaCompletionRelay(ConnectionCameraConsumerCommands, lease, { true }, Dispatchers.Unconfined, value)
            }
        }
    }

    @Test fun t948RejectedAndThrowingTerminalCommandsRetryOneOriginalProof() = runBlocking {
        val attempts = AtomicInteger()
        val accepted = CompletableDeferred<Unit>()
        val commands = java.util.concurrent.CopyOnWriteArrayList<ConnectionWorkerCommand>()
        val relay = ConnectionMediaCompletionRelay(ConnectionCameraConsumerCommands, lease, {
            commands.add(it)
            when (attempts.incrementAndGet()) {
                1 -> false
                2 -> throw IllegalStateException("bounded queue full")
                else -> true
            }
        }, Dispatchers.Default, delivered = { accepted.complete(Unit) })
        try {
            relay.ready(); relay.ready()
            withTimeout(2000) { accepted.await() }
            assertEquals(3, attempts.get())
            assertEquals(List(3) { ConnectionWorkerCommand.CameraConsumerRetired(lease) }, commands)
            relay.ready(); assertEquals(3, attempts.get())
        } finally { relay.retire() }
        val stopped = ConnectionMediaCompletionRelay(ConnectionCameraConsumerCommands, lease, { throw IllegalStateException("stop rejected") }, Dispatchers.Unconfined)
        assertFalse(stopped.stop()); stopped.retire()
    }

    @Test fun t948ActualRetirementJoinsBlockedOriginalRelayRatherThanLosingIt() = runBlocking {
        val entered = CountDownLatch(1)
        val release = CountDownLatch(1)
        val relay = ConnectionMediaCompletionRelay(ConnectionCameraConsumerCommands, lease, {
            entered.countDown(); check(release.await(4, TimeUnit.SECONDS)); false
        })
        relay.ready()
        assertTrue(entered.await(2, TimeUnit.SECONDS))
        val retiring = async(Dispatchers.Default) { relay.retire() }
        try {
            delay(40); assertFalse("T948 blocked sender remains owned", retiring.isCompleted)
            assertFalse(relay.stop())
        } finally { release.countDown(); withTimeout(2000) { retiring.await() } }
    }

    @Test fun t948BoundedClaimOrderPropertyPublishesExactlyOneProof() = runBlocking {
        val random = Random(948)
        repeat(128) {
            val commands = mutableListOf<ConnectionWorkerCommand>()
            val owner = ConnectionMediaConsumerOwner(route().lease, ConnectionCameraConsumerCommands, { commands.add(it); true }, Dispatchers.Unconfined)
            val claims = List(random.nextInt(16)) { checkNotNull(owner.retain()) }.shuffled(kotlin.random.Random(random.nextInt()))
            try {
                val closeFirst = random.nextBoolean()
                if (closeFirst) owner.closed(lease)
                claims.forEach { claim ->
                    claim.retired(if (random.nextBoolean()) ResourceRetirement.RETIRED else ResourceRetirement.NOT_STARTED)
                    claim.retired(ResourceRetirement.RETIRED)
                }
                if (!closeFirst) { assertTrue(commands.isEmpty()); owner.closed(lease) }
                owner.closed(lease)
                assertEquals(listOf(ConnectionWorkerCommand.CameraConsumerRetired(lease)), commands)
                assertNull(owner.retain())
            } finally { owner.retire() }
        }
    }

    @Test fun t948ProjectionPreservesCanonicalProfileAndCredential() {
        val owner = ConnectionMediaConsumerOwner(route().lease, ConnectionCameraConsumerCommands, { true }, Dispatchers.Unconfined)
        val endpoint = CameraCaptureTarget(ConnectionCameraTarget.endpoint(route()), owner).endpoint
        assertEquals("000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f", endpoint.token)
        assertEquals(CameraEndpoint(endpoint.token, 12345, 640, 480, 30, 3000, CameraLens.REAR, true, 150, true, 1000), endpoint)
        assertTrue(endpoint.valid())
        runBlocking { owner.retire() }
    }
}
