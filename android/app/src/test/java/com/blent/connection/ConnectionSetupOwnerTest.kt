package com.blent.connection

import org.junit.Assert.*
import org.junit.Test
import java.security.MessageDigest
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicLong

internal class SetupBackend : ConnectionSetupBackend {
    val local = ConnectionIdentity(byteArrayOf(1))
    val remote = ConnectionIdentity(byteArrayOf(2))
    val entered = CountDownLatch(1)
    var authenticateGate = CountDownLatch(0)
    var readyGate = CountDownLatch(0)
    val readyEntered = CountDownLatch(1)
    var discoverGate = CountDownLatch(0)
    val discoverEntered = CountDownLatch(1)
    var closeGate = CountDownLatch(0)
    var ioGate = CountDownLatch(0)
    val ioEntered = CountDownLatch(1)
    var closeFailure: Exception? = null
    var proof = setupPeer(remote)
    var capacity = 4096
    @Volatile var closed = false
    @Volatile var ioMade = false
    val stream = SetupIo()
    override fun localIdentity() = local
    override fun discover(timeoutMs: Long): ConnectionIdentity {
        discoverEntered.countDown(); check(discoverGate.await(3, TimeUnit.SECONDS)); return remote
    }
    override fun ready(peer: ConnectionIdentity, readinessTimeoutMs: Long) {
        readyEntered.countDown(); check(readyGate.await(3, TimeUnit.SECONDS))
    }
    override fun authenticate(peer: ConnectionIdentity): ConnectionPeer {
        entered.countDown(); check(authenticateGate.await(3, TimeUnit.SECONDS)); return proof
    }
    override fun negotiate(capacity: Int, timeoutMs: Long) = this.capacity
    override fun io(wake: () -> Unit): ConnectionIo {
        ioEntered.countDown(); check(ioGate.await(3, TimeUnit.SECONDS)); ioMade = true; return stream
    }
    override fun close() {
        check(closeGate.await(3, TimeUnit.SECONDS))
        closeFailure?.let { throw it }
        stream.stop(); closed = true
    }
    override fun retired() = closed
}

internal class SetupIo : ConnectionIo {
    @Volatile var stopped = false
    override fun submit(request: ConnectionIoRequest) = false
    override fun poll(): ConnectionIoCompletion? = null
    override fun stop() { stopped = true }
    override fun retired() = stopped
    override fun closeFailure(): Exception? = null
}

internal class SetupStore : PeerTrustStore {
    var bytes: ByteArray? = null
    var gate = CountDownLatch(0)
    val entered = CountDownLatch(1)
    var writeFailure: Exception? = null
    var closeFailure: Exception? = null
    @Volatile var closed = false
    override fun load() = bytes
    override fun replace(record: ByteArray) {
        entered.countDown(); check(gate.await(3, TimeUnit.SECONDS))
        writeFailure?.let { throw it }; bytes = record.copyOf()
    }
    override fun close() { closed = true; closeFailure?.let { throw it } }
}

internal fun setupPeer(identity: ConnectionIdentity): ConnectionPeer = ConnectionPeer.parse(
    MessageDigest.getInstance("SHA-256").digest(identity.certificate()).joinToString("") { "%02x".format(it.toInt() and 255) },
)

internal fun setupAwait(condition: () -> Boolean) {
    val deadline = System.nanoTime() + TimeUnit.SECONDS.toNanos(3)
    while (!condition()) { check(System.nanoTime() < deadline) { "Setup test deadline" }; Thread.sleep(1) }
}

internal class SetupRig(
    val backend: SetupBackend = SetupBackend(),
    val store: SetupStore = SetupStore(),
    wake: () -> Unit = {},
    limits: ConnectionSetupLimits = ConnectionSetupLimits(4096, 1000, 100, 50),
) : AutoCloseable {
    val clock = AtomicLong(10)
    val owner = ConnectionSetupOwner(backend, { store }, clock::get, limits, wake)
    val events = mutableListOf<ConnectionWorkerEvent>()
    fun drain() { while (true) { events.add(owner.poll() ?: return) } }
    fun ticket(): ConnectionTicket {
        setupAwait { drain(); events.any { it is ConnectionWorkerEvent.Pairing } }
        return (events.first { it is ConnectionWorkerEvent.Pairing } as ConnectionWorkerEvent.Pairing).ticket
    }
    fun ready(): ConnectionSetupReady {
        var result: ConnectionSetupReady? = null
        setupAwait { result = owner.takeReady(); result != null }
        return checkNotNull(result)
    }
    override fun close() {
        backend.authenticateGate.countDown(); backend.readyGate.countDown(); backend.discoverGate.countDown(); backend.closeGate.countDown(); backend.ioGate.countDown(); store.gate.countDown()
        owner.stop(); setupAwait { owner.retired() || owner.terminationRequired() }
    }
}

class ConnectionSetupOwnerTest {
    @Test fun t895RememberedPeerMetadataAllowsRevocationWithoutGrantingTlsProof() {
        val backend = SetupBackend().apply { authenticateGate = CountDownLatch(1) }
        val store = SetupStore()
        val remote = setupPeer(backend.remote)
        store.bytes = ConnectionTrustCodec.encode(TrustSnapshot(setupPeer(backend.local), 1, listOf(remote)))
        SetupRig(backend, store).use { rig ->
            assertTrue(backend.entered.await(2, TimeUnit.SECONDS)); rig.drain()
            val metadata = rig.events.filterIsInstance<ConnectionWorkerEvent.Peer>()
            assertEquals("T895 remembered peer must remain identifiable before proof", 1, metadata.size)
            assertEquals(remote, metadata.single().ticket.peer)
            assertEquals(setupPeer(backend.local), metadata.single().local)
            assertTrue(metadata.single().remembered)
            assertNull(rig.owner.takeReady())
            assertFalse(rig.events.any { it is ConnectionWorkerEvent.Pairing })
            backend.authenticateGate.countDown()
            val ready = rig.ready()
            assertEquals(metadata.single().ticket, ready.approval.ticket)
            assertTrue(rig.owner.revoke(metadata.single().ticket.peer))
            assertFalse(rig.owner.current(ready.approval))
            setupAwait { rig.drain(); rig.events.any { it is ConnectionWorkerEvent.Trust && !it.remembered } }
            assertTrue(ConnectionTrustCodec.decode(checkNotNull(store.bytes)).peers.isEmpty())
        }
    }

    @Test fun t886ApprovalProofWindowAndBorrowedIoHaveOneCurrentOwner() {
        SetupRig().use { rig ->
            val ticket = rig.ticket()
            assertFalse(rig.owner.decide(ticket.copy(epoch = ticket.epoch + 1), true, false))
            assertTrue(rig.owner.decide(ticket, true, true))
            assertFalse(rig.owner.decide(ticket, true, true))
            val ready = rig.ready()
            assertSame(rig.backend.stream, ready.io); assertEquals(4096, ready.capacity)
            assertTrue(rig.owner.current(ready.approval)); assertNull(rig.owner.takeReady())
            setupAwait { rig.drain(); rig.events.any { it is ConnectionWorkerEvent.Trust && it.remembered } }
            rig.owner.stop(); assertFalse(rig.owner.current(ready.approval))
            assertFalse(rig.owner.revoke(ticket.peer)); assertFalse(rig.owner.decide(ticket, true, false))
        }
    }

    @Test fun t886PersistedTrustStillRequiresTlsProof() {
        val backend = SetupBackend(); val store = SetupStore()
        store.bytes = ConnectionTrustCodec.encode(TrustSnapshot(setupPeer(backend.local), 1, listOf(setupPeer(backend.remote))))
        SetupRig(backend, store).use { rig ->
            val ready = rig.ready(); rig.drain()
            assertFalse(rig.events.any { it is ConnectionWorkerEvent.Pairing })
            assertTrue(rig.owner.current(ready.approval)); assertEquals(0, backend.entered.count)
        }
    }

    @Test fun t886RevokeDuringRememberImmediatelyInvalidatesThenDeletesDurably() {
        val store = SetupStore().apply { gate = CountDownLatch(1) }
        SetupRig(store = store).use { rig ->
            val ticket = rig.ticket(); assertTrue(rig.owner.decide(ticket, true, true))
            assertTrue(store.entered.await(2, TimeUnit.SECONDS))
            assertTrue(rig.owner.revoke(ticket.peer)); assertNull(rig.owner.takeReady())
            assertFalse(rig.owner.retired()); rig.drain()
            assertFalse(rig.events.any { it is ConnectionWorkerEvent.Trust })
            store.gate.countDown(); setupAwait { store.closed }; rig.drain()
            assertEquals(listOf(true, false), rig.events.filterIsInstance<ConnectionWorkerEvent.Trust>().map { it.remembered })
            assertTrue(ConnectionTrustCodec.decode(checkNotNull(store.bytes)).peers.isEmpty())
        }
    }

    @Test fun t886CancellationCannotPublishLateProofAndRetainsStuckNativeWork() {
        val backend = SetupBackend().apply { authenticateGate = CountDownLatch(1) }
        SetupRig(backend).use { rig ->
            val ticket = rig.ticket(); rig.owner.decide(ticket, true, false)
            assertTrue(backend.entered.await(2, TimeUnit.SECONDS)); rig.owner.stop()
            rig.clock.set(100); rig.owner.advance()
            assertTrue(rig.owner.terminationRequired()); assertFalse(rig.owner.retired())
            backend.authenticateGate.countDown(); setupAwait { rig.owner.retired() }
            assertNull(rig.owner.takeReady()); assertFalse(backend.ioMade)
        }
    }

    @Test fun t886ApprovalExpiresWithoutNativeWorkerProgress() {
        SetupRig().use { rig ->
            rig.ticket(); rig.clock.set(1010); rig.owner.advance()
            setupAwait { rig.owner.retired() }; assertNotNull(rig.owner.failure())
        }
    }

    @Test fun t886DeniedAndWrongTlsProofCannotAdmit() {
        SetupRig().use { rig ->
            assertTrue(rig.owner.decide(rig.ticket(), false, false))
            setupAwait { rig.owner.retired() }; assertNull(rig.owner.takeReady())
        }
        val backend = SetupBackend().apply { proof = setupPeer(local) }
        SetupRig(backend).use { rig ->
            rig.owner.decide(rig.ticket(), true, false)
            setupAwait { rig.owner.retired() }; assertNotNull(rig.owner.failure())
        }
    }

    @Test fun t886StorageFailurePreventsAdmissionAndReportsFailure() {
        val store = SetupStore().apply { writeFailure = IllegalStateException("disk full") }
        SetupRig(store = store).use { rig ->
            rig.owner.decide(rig.ticket(), true, true)
            setupAwait { rig.owner.retired() }; assertNull(rig.owner.takeReady())
            assertNotNull(rig.owner.failure())
        }
    }

    @Test fun t886CloseStallAndFailureAreNotRetirement() {
        val backend = SetupBackend().apply { closeGate = CountDownLatch(1) }
        SetupRig(backend).use { rig ->
            rig.ticket(); rig.owner.stop(); rig.clock.set(100); rig.owner.advance()
            assertTrue(rig.owner.terminationRequired()); assertFalse(rig.owner.retired())
            backend.closeGate.countDown(); setupAwait { rig.owner.retired() }
        }
        val failing = SetupBackend().apply { closeFailure = IllegalStateException("close") }
        val rig = SetupRig(failing)
        try {
            rig.ticket(); rig.owner.stop(); setupAwait { rig.owner.failure() != null }
            setupAwait { rig.owner.terminationRequired() }; assertFalse(rig.owner.retired())
        } finally { failing.closed = true; rig.close() }
    }

    @Test fun t886MalformedNegotiatedCapacityAndClockFailClosed() {
        SetupRig(SetupBackend().apply { capacity = 4097 }).use { rig ->
            rig.owner.decide(rig.ticket(), true, false)
            setupAwait { rig.owner.retired() }; assertNotNull(rig.owner.failure())
        }
        SetupRig().use { rig -> rig.ticket(); rig.clock.set(9); rig.owner.advance(); setupAwait { rig.owner.retired() } }
    }

    @Test fun t886UnrelatedDurableRevokeKeepsCurrentPeerAndRejectsDuplicateQueuedWork() {
        val backend = SetupBackend(); val store = SetupStore()
        val other = ConnectionPeer.parse("a".repeat(64))
        store.bytes = ConnectionTrustCodec.encode(TrustSnapshot(setupPeer(backend.local), 1, listOf(other)))
        SetupRig(backend, store).use { rig ->
            rig.owner.decide(rig.ticket(), true, false); val ready = rig.ready()
            assertTrue(rig.owner.revoke(other))
            setupAwait { rig.drain(); rig.events.any { it is ConnectionWorkerEvent.Trust && !it.remembered } }
            assertTrue(rig.owner.current(ready.approval))
        }
    }

    @Test fun t886LimitsRejectBoundedMutations() {
        val random = java.util.Random(886)
        repeat(256) {
            val value = random.nextInt(8192) - 4096
            assertThrows(IllegalArgumentException::class.java) { ConnectionSetupLimits(capacity = value) }
        }
        assertThrows(IllegalArgumentException::class.java) { ConnectionSetupLimits(approvalTimeoutMs = 0) }
        assertThrows(IllegalArgumentException::class.java) { ConnectionSetupLimits(nativeTimeoutMs = 60001) }
        assertThrows(IllegalArgumentException::class.java) { ConnectionSetupLimits(retirementTimeoutMs = -1) }
    }
    @Test fun t886VerifiedProofUsesNativeDeadlineDuringDurableRemember() {
        val store = SetupStore().apply { gate = CountDownLatch(1) }
        SetupRig(store = store).use { rig ->
            rig.owner.decide(rig.ticket(), true, true)
            assertTrue(store.entered.await(2, TimeUnit.SECONDS))
            rig.clock.set(111); rig.owner.advance()
            assertNotNull("Native storage deadline must not inherit human approval timeout", rig.owner.failure())
            assertNull(rig.owner.takeReady()); assertFalse(rig.owner.retired())
        }
    }

    @Test fun t886LateIoCreationRemainsOwnedAndStoppedAfterCancellation() {
        val backend = SetupBackend().apply { ioGate = CountDownLatch(1) }
        SetupRig(backend).use { rig ->
            rig.owner.decide(rig.ticket(), true, false)
            assertTrue(backend.ioEntered.await(2, TimeUnit.SECONDS))
            rig.owner.stop(); setupAwait { backend.closed }
            assertFalse(rig.owner.retired())
            backend.ioGate.countDown(); setupAwait { rig.owner.retired() }
            assertTrue(backend.stream.stopped); assertNull(rig.owner.takeReady())
        }
    }

    @Test fun t886PendingRevokesAndUnreadEventsStayBounded() {
        val backend = SetupBackend().apply { authenticateGate = CountDownLatch(1) }
        SetupRig(backend).use { rig ->
            rig.owner.decide(rig.ticket(), true, false)
            assertTrue(backend.entered.await(2, TimeUnit.SECONDS))
            repeat(32) { assertTrue(rig.owner.revoke(ConnectionPeer.parse("%064x".format(it)))) }
            assertFalse(rig.owner.revoke(ConnectionPeer.parse("0".repeat(64))))
            assertFalse(rig.owner.revoke(ConnectionPeer.parse("f".repeat(64))))
            backend.authenticateGate.countDown(); setupAwait { rig.owner.retired() }
            assertNotNull(rig.owner.failure()); rig.drain()
            assertTrue(rig.events.size <= 35)
        }
    }

    @Test fun t886WakeAndTrustCloseFailuresCannotEscapeNativeOwnership() {
        val store = SetupStore().apply { closeFailure = IllegalStateException("close store") }
        SetupRig(store = store).use { rig ->
            rig.ticket(); rig.owner.stop(); setupAwait { rig.owner.retired() }; assertNotNull(rig.owner.failure())
        }
        SetupRig(wake = { throw IllegalStateException("wake") }).use { rig ->
            setupAwait { rig.owner.retired() }; assertNotNull(rig.owner.failure())
        }
    }

    @Test fun t886RevocationBeforeHelloCannotAdmitMatchingLaterCandidate() {
        val backend = SetupBackend().apply { discoverGate = CountDownLatch(1) }
        val store = SetupStore()
        store.bytes = ConnectionTrustCodec.encode(TrustSnapshot(setupPeer(backend.local), 1, listOf(setupPeer(backend.remote))))
        SetupRig(backend, store).use { rig ->
            assertTrue(backend.discoverEntered.await(2, TimeUnit.SECONDS))
            assertTrue(rig.owner.revoke(setupPeer(backend.remote)))
            backend.discoverGate.countDown()
            setupAwait { rig.drain(); rig.events.any { it is ConnectionWorkerEvent.Trust && !it.remembered } }
            assertNull("Revoked candidate must not publish ready", rig.owner.takeReady())
            assertFalse("Revoked candidate must not reach native TLS", backend.ioMade)
        }
    }

    @Test fun t886CancelDuringSuccessfulSaveReportsDurableTruthWithoutAdmission() {
        val store = SetupStore().apply { gate = CountDownLatch(1) }
        SetupRig(store = store).use { rig ->
            rig.owner.decide(rig.ticket(), true, true)
            assertTrue(store.entered.await(2, TimeUnit.SECONDS))
            rig.owner.stop(); store.gate.countDown(); setupAwait { rig.owner.retired() }; rig.drain()
            assertNull(rig.owner.takeReady())
            assertEquals(listOf(true), rig.events.filterIsInstance<ConnectionWorkerEvent.Trust>().map { it.remembered })
            assertEquals(listOf(setupPeer(rig.backend.remote)), ConnectionTrustCodec.decode(checkNotNull(store.bytes)).peers)
        }
    }

    @Test fun t886ConfiguredTrustCapacityAcceptsExpandedSnapshotWithoutEviction() {
        val backend = SetupBackend(); val store = SetupStore()
        val peers = (List(64) { ConnectionPeer.parse("%064x".format(it)) } + setupPeer(backend.remote)).sortedBy { it.fingerprint }
        store.bytes = ConnectionTrustCodec.encode(TrustSnapshot(setupPeer(backend.local), 1, peers))
        SetupRig(backend, store, limits = ConnectionSetupLimits(4096, 1000, 100, 50, 65)).use { rig ->
            val ready = rig.ready(); assertTrue(rig.owner.current(ready.approval))
            assertEquals(peers, ConnectionTrustCodec.decode(checkNotNull(store.bytes)).peers)
        }
        assertThrows(IllegalArgumentException::class.java) { ConnectionSetupLimits(trustedPeers = 0) }
        assertThrows(IllegalArgumentException::class.java) { ConnectionSetupLimits(trustedPeers = 1025) }
        assertEquals(1024, ConnectionSetupLimits(trustedPeers = 1024).trustedPeers)
    }

    @Test fun t886ReadyCompletedTlsUsesNativeDeadlineRatherThanHumanApprovalBudget() {
        val backend = SetupBackend().apply { authenticateGate = CountDownLatch(1) }
        SetupRig(backend).use { rig ->
            rig.owner.decide(rig.ticket(), true, false)
            assertTrue(backend.entered.await(2, TimeUnit.SECONDS))
            rig.clock.set(111); rig.owner.advance()
            assertNotNull("Completed READY must start a native TLS deadline", rig.owner.failure())
            assertNull(rig.owner.takeReady()); assertFalse(rig.owner.retired())
            rig.clock.set(162); rig.owner.advance(); assertTrue(rig.owner.terminationRequired())
        }
    }

    @Test fun t886ReadyAtApprovalExpiryCannotStartTls() {
        val backend = SetupBackend().apply { readyGate = CountDownLatch(1) }
        SetupRig(backend).use { rig ->
            rig.owner.decide(rig.ticket(), true, false)
            assertTrue(backend.readyEntered.await(2, TimeUnit.SECONDS))
            rig.clock.set(1010); backend.readyGate.countDown()
            setupAwait { rig.owner.retired() }
            assertEquals(1L, backend.entered.count); assertNotNull(rig.owner.failure())
        }
    }

    @Test fun t886ReadyNearExpiryUsesShorterApprovalBudgetForTls() {
        val backend = SetupBackend().apply { readyGate = CountDownLatch(1); authenticateGate = CountDownLatch(1) }
        SetupRig(backend).use { rig ->
            rig.owner.decide(rig.ticket(), true, false)
            assertTrue(backend.readyEntered.await(2, TimeUnit.SECONDS))
            rig.clock.set(1009); backend.readyGate.countDown()
            assertTrue(backend.entered.await(2, TimeUnit.SECONDS))
            rig.clock.set(1010); rig.owner.advance()
            assertNotNull(rig.owner.failure()); assertNull(rig.owner.takeReady())
        }
    }

    @Test fun t886LateTlsCompletionCannotResetDeadlineBeforeReactorTick() {
        val backend = SetupBackend().apply { authenticateGate = CountDownLatch(1) }
        SetupRig(backend).use { rig ->
            rig.owner.decide(rig.ticket(), true, false)
            assertTrue(backend.entered.await(2, TimeUnit.SECONDS))
            rig.clock.set(111); backend.authenticateGate.countDown()
            setupAwait { backend.ioMade || rig.owner.failure() != null }
            assertNull("Expired TLS cannot publish Ready before timer tick", rig.owner.takeReady())
            assertNotNull(rig.owner.failure())
        }
    }

    @Test fun t886LateDurableSaveReportsTruthButCannotResetNativeDeadline() {
        val store = SetupStore().apply { gate = CountDownLatch(1) }
        SetupRig(store = store).use { rig ->
            rig.owner.decide(rig.ticket(), true, true)
            assertTrue(store.entered.await(2, TimeUnit.SECONDS))
            rig.clock.set(111); store.gate.countDown()
            setupAwait { rig.backend.ioMade || rig.owner.failure() != null }; rig.drain()
            assertNull("Expired persistence cannot publish Ready before timer tick", rig.owner.takeReady())
            assertEquals(listOf(true), rig.events.filterIsInstance<ConnectionWorkerEvent.Trust>().map { it.remembered })
            assertNotNull(rig.owner.failure())
        }
    }

    @Test fun t886LateIoCreationCannotPublishReadyBeforeTimerTick() {
        val backend = SetupBackend().apply { ioGate = CountDownLatch(1) }
        SetupRig(backend).use { rig ->
            val ticket = rig.ticket(); rig.owner.decide(ticket, true, false)
            assertTrue(backend.ioEntered.await(2, TimeUnit.SECONDS))
            rig.clock.set(111); backend.ioGate.countDown()
            setupAwait { rig.owner.current(ConnectionApproval(ticket, false)) || rig.owner.failure() != null }
            assertNull("Expired native handle stays owned but cannot admit", rig.owner.takeReady())
            setupAwait { rig.owner.retired() }; assertTrue(backend.stream.stopped)
        }
    }

    @Test fun t886LateHelloCannotReplaceExpiredNativeDeadlineWithApprovalBudget() {
        val backend = SetupBackend().apply { discoverGate = CountDownLatch(1) }
        SetupRig(backend).use { rig ->
            assertTrue(backend.discoverEntered.await(2, TimeUnit.SECONDS))
            rig.clock.set(111); backend.discoverGate.countDown()
            setupAwait { rig.drain(); rig.events.any { it is ConnectionWorkerEvent.Pairing } || rig.owner.failure() != null }
            assertNotNull("Expired HELLO must not start human approval", rig.owner.failure())
            assertFalse(rig.events.any { it is ConnectionWorkerEvent.Pairing })
        }
    }

    @Test fun t886UnrelatedDurableRevokeHasNativeDeadlineWhileCurrentPeerRemainsReady() {
        val backend = SetupBackend(); val store = SetupStore()
        val other = ConnectionPeer.parse("a".repeat(64))
        store.bytes = ConnectionTrustCodec.encode(TrustSnapshot(setupPeer(backend.local), 1, listOf(other)))
        SetupRig(backend, store).use { rig ->
            rig.owner.decide(rig.ticket(), true, false); val ready = rig.ready()
            store.gate = CountDownLatch(1)
            assertTrue(rig.owner.revoke(other)); assertTrue(store.entered.await(2, TimeUnit.SECONDS))
            assertTrue(rig.owner.current(ready.approval))
            rig.clock.set(111); rig.owner.advance()
            assertNotNull("Unrelated trust deletion needs native timeout", rig.owner.failure())
            assertFalse(rig.owner.current(ready.approval)); assertFalse(rig.owner.retired())
            store.gate.countDown(); setupAwait { rig.owner.retired() }; rig.drain()
            assertEquals(listOf(false), rig.events.filterIsInstance<ConnectionWorkerEvent.Trust>().map { it.remembered })
        }
    }

    @Test fun t886StopGraceBeginsAtActualStopRatherThanPreviousReactorTick() {
        val backend = SetupBackend().apply { closeGate = CountDownLatch(1) }
        SetupRig(backend).use { rig ->
            rig.owner.decide(rig.ticket(), true, false); rig.ready()
            rig.clock.set(100); rig.owner.stop(); rig.owner.advance()
            assertFalse("Native closer retains full grace from stop time", rig.owner.terminationRequired())
            rig.clock.set(149); rig.owner.advance(); assertFalse(rig.owner.terminationRequired())
            rig.clock.set(150); rig.owner.advance(); assertTrue(rig.owner.terminationRequired())
        }
    }

}
