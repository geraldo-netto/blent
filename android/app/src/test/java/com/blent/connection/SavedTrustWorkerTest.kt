package com.blent.connection

import kotlinx.coroutines.*
import org.junit.Assert.*
import org.junit.Test
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import java.nio.file.Files
import java.nio.file.attribute.PosixFilePermissions
import java.nio.channels.OverlappingFileLockException

class SavedTrustWorkerTest {
    private fun peer(index: Int) = ConnectionPeer.parse(index.toString(16).padStart(64, '0'))
    private class Store(var record: ByteArray? = null) : PeerTrustStore {
        var closed = 0
        var failClose = false
        var failWrite = false
        override fun load(): ByteArray? = record?.copyOf()
        override fun replace(record: ByteArray) { check(!failWrite); this.record = record.copyOf() }
        override fun close() { closed++; check(!failClose) }
    }
    private fun store(count: Int) = Store(ConnectionTrustCodec.encode(TrustSnapshot(peer(0), 1, (1..count).map(::peer))))
    @Test fun t1151ListAndConditionalForgetReuseExclusiveTrustOwnerWithoutGrant() = runBlocking<Unit> {
        val store = store(1024)
        val events = mutableListOf<ConnectionWorkerEvent>()
        val worker = SavedTrustWorker(SavedTrustRequest.List, { SavedTrustAccesses.open(store) { peer(0) } }, events::add)
        worker.execute(); worker.retire()
        assertEquals(1, store.closed)
        assertEquals(22, events.size)
        assertEquals(1024, events.map { (it as ConnectionWorkerEvent.SavedComputers).page.peers.size }.sum())
        val selected = SavedComputerSelection(peer(0), 1, peer(2))
        val forgetting = SavedTrustWorker(SavedTrustRequest.Forget(selected), { SavedTrustAccesses.open(store) { peer(0) } }, {})
        forgetting.execute(); forgetting.retire()
        val stored = ConnectionTrustCodec.decode(store.record!!)
        assertEquals(2L, stored.revision); assertFalse(stored.peers.contains(peer(2)))
        assertEquals(2, store.closed)
        assertFalse(forgetting.command(ConnectionWorkerCommand.Revoke(peer(1))))
    }
    @Test fun t1151UninitializedIdentityDoesNotCreateKeysOrDiscardExistingTrust() = runBlocking<Unit> {
        val empty = Store()
        val values = mutableListOf<ConnectionWorkerEvent>()
        val worker = SavedTrustWorker(SavedTrustRequest.List, { SavedTrustAccesses.open(empty) { null } }, values::add)
        worker.execute(); worker.retire()
        val page = (values.single() as ConnectionWorkerEvent.SavedComputers).page
        assertNull(page.local); assertEquals(0, page.total); assertNull(empty.record)
        assertEquals(1, empty.closed)
        val existing = store(1)
        assertThrows(IllegalStateException::class.java) { SavedTrustAccesses.open(existing) { null } }
        assertEquals(1, existing.closed)
        assertEquals(1, ConnectionTrustCodec.decode(existing.record!!).peers.size)
        val access = SavedTrustAccesses.open(Store()) { null }
        assertThrows(IllegalStateException::class.java) { access.forget(SavedComputerSelection(peer(0), 1, peer(1))) }
        access.close(); access.close()
        assertThrows(IllegalStateException::class.java) { access.snapshot() }
        assertThrows(IllegalStateException::class.java) { access.forget(SavedComputerSelection(peer(0), 1, peer(1))) }
    }
    @Test fun t1151IdentityAndStorageFailuresRetainCleanupAndNeverReportSuccess() = runBlocking<Unit> {
        val store = Store().apply { failClose = true }
        val error = assertThrows(IllegalStateException::class.java) {
            SavedTrustAccesses.open(store) { error("identity unavailable") }
        }
        assertEquals(1, store.closed); assertEquals(1, error.suppressed.size)
        val corrupt = Store(byteArrayOf(1))
        assertThrows(IllegalArgumentException::class.java) { SavedTrustAccesses.open(corrupt) { peer(0) } }
        assertEquals(1, corrupt.closed)
        val failed = SavedTrustWorker(SavedTrustRequest.List, { error("factory ownership unknown") }, {})
        assertTrue(runCatching { failed.execute() }.isFailure)
        assertTrue(runCatching { failed.retire() }.isFailure)
        val closed = Store().apply { failClose = true }
        val close = SavedTrustWorker(SavedTrustRequest.List, { SavedTrustAccesses.open(closed) { peer(0) } }, {})
        assertTrue(runCatching { close.execute() }.isFailure)
        assertTrue(runCatching { close.retire() }.isFailure)
    }
    @Test fun t1151CancellationDuringNativeOpenKeepsRealStoreLockedUntilActualReturn() = runBlocking<Unit> {
        val root = Files.createTempDirectory("blent-t1151-", PosixFilePermissions.asFileAttribute(PosixFilePermissions.fromString("rwx------")))
        val entered = CountDownLatch(1); val release = CountDownLatch(1)
        val values = mutableListOf<ConnectionWorkerEvent>()
        val worker = SavedTrustWorker(SavedTrustRequest.List, {
            SavedTrustAccesses.open(AndroidPeerTrustStore.open(root)) {
                entered.countDown(); release.await(); peer(0)
            }
        }, values::add)
        val execution = async(Dispatchers.IO) { runCatching { worker.execute() } }
        try {
            assertTrue(entered.await(5, TimeUnit.SECONDS))
            worker.cancel(); execution.cancel()
            assertThrows(OverlappingFileLockException::class.java) { AndroidPeerTrustStore.open(root) }
            assertFalse(execution.isCompleted)
        } finally {
            release.countDown()
            execution.join()
            worker.retire()
            AndroidPeerTrustStore.open(root).close()
            Files.walk(root).use { paths -> paths.sorted(Comparator.reverseOrder()).forEach(Files::delete) }
        }
        assertTrue(values.isEmpty())
    }
    @Test fun t1151CancelledUnstartedJobAndReporterFailureHaveActualCleanup() = runBlocking<Unit> {
        var created = 0
        val unstarted = SavedTrustWorker(SavedTrustRequest.List, { created++; error("unexpected") }, {})
        unstarted.retire()
        assertTrue(runCatching { unstarted.execute() }.isFailure)
        assertEquals(0, created)
        val cancelled = SavedTrustWorker(SavedTrustRequest.List, { created++; error("unexpected") }, {})
        cancelled.cancel(); cancelled.execute(); cancelled.retire()
        assertEquals(0, created)
        val store = store(49)
        val failing = SavedTrustWorker(SavedTrustRequest.List, { SavedTrustAccesses.open(store) { peer(0) } }, { error("reporter failed") })
        assertTrue(runCatching { failing.execute() }.isFailure)
        failing.retire(); assertEquals(1, store.closed)
    }
}
