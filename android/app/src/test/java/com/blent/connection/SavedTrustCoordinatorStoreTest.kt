package com.blent.connection

import com.blent.LifecycleRun
import java.util.concurrent.CountDownLatch
import kotlinx.coroutines.*
import org.junit.Assert.*
import org.junit.Test

class SavedTrustCoordinatorStoreTest {
    private class Store : PeerTrustStore {
        var record = ConnectionTrustCodec.encode(TrustSnapshot(savedPeer(0), 1, (1..1024).map(::savedPeer)))
        var closes = 0
        var failWrite = false
        var closeEntered: CountDownLatch? = null
        var closeRelease: CountDownLatch? = null
        override fun load() = record.copyOf()
        override fun replace(record: ByteArray) { check(!failWrite) { "write failed" }; this.record = record.copyOf() }
        override fun close() { closeEntered?.countDown(); closeRelease?.await(); closes++ }
    }
    private class Fixture {
        val store = Store()
        val scope = CoroutineScope(SupervisorJob() + Dispatchers.Unconfined)
        val access = ConnectionAccess()
        val binding = SavedTrustBinding(scope, access, SavedTrustRunFactory { request, report ->
            val raw = SavedTrustWorker(request, { SavedTrustAccesses.open(store) { savedPeer(0) } },
                { report((it as ConnectionWorkerEvent.SavedComputers).page) })
            object : LifecycleRun {
                override suspend fun execute() = withContext(Dispatchers.IO) { raw.execute() }
                override fun cancel() = raw.cancel()
                override suspend fun retire() = raw.retire()
            }
        })
        val client = binding.attach()
        suspend fun settled() {
            withTimeout(5000) {
                while (binding.state.value.phase in setOf(SavedTrustPhase.BUSY, SavedTrustPhase.RETIRING)) yield()
            }
        }
        suspend fun close() {
            store.closeRelease?.countDown(); client.close(); scope.cancel(); scope.coroutineContext[Job]!!.join()
        }
    }

    @Test fun t1168JoinedStoreForgetPreservesOtherPeersAndRejectsExternalRevisionChanges() = runBlocking {
        val f = Fixture()
        try {
            assertTrue(f.client.refresh()); f.settled()
            assertEquals(1024, f.binding.state.value.computers!!.peers.size)
            val stale = checkNotNull(f.client.choice(savedPeer(1)))
            f.store.record = ConnectionTrustCodec.encode(TrustSnapshot(savedPeer(0), 2, (1..1024).map(::savedPeer)))
            val previous = f.store.record.copyOf()
            assertTrue(f.client.forget(stale)); f.settled()
            assertEquals(SavedTrustPhase.FAILED, f.binding.state.value.phase); assertArrayEquals(previous, f.store.record)
            assertTrue(f.client.refresh()); f.settled()
            assertTrue(f.client.forget(checkNotNull(f.client.choice(savedPeer(1))))); f.settled()
            val result = ConnectionTrustCodec.decode(f.store.record)
            assertEquals(3L, result.revision); assertEquals((2..1024).map(::savedPeer), result.peers)
            assertEquals(4, f.store.closes); assertNull(f.access.current())
        } finally { f.close() }
    }

    @Test fun t1168StorageWriteFailurePreservesBytesAndNoPartialListBecomesReady() = runBlocking {
        val f = Fixture()
        try {
            assertTrue(f.client.refresh()); f.settled()
            val before = f.store.record.copyOf(); f.store.failWrite = true
            assertTrue(f.client.forget(checkNotNull(f.client.choice(savedPeer(1024))))); f.settled()
            assertEquals(SavedTrustPhase.FAILED, f.binding.state.value.phase)
            assertEquals("write failed", f.binding.state.value.failure)
            assertNull(f.binding.state.value.computers); assertArrayEquals(before, f.store.record)
            assertEquals(2, f.store.closes); assertNull(f.access.current())
        } finally { f.close() }
    }

    @Test fun t1168UiLossCannotReleaseOriginalSynchronousStoreClose() = runBlocking {
        val f = Fixture()
        f.store.closeEntered = CountDownLatch(1); f.store.closeRelease = CountDownLatch(1)
        try {
            assertTrue(f.client.refresh())
            withContext(Dispatchers.IO) { f.store.closeEntered!!.await() }
            f.client.close(); val replacement = f.binding.attach()
            assertFalse(replacement.refresh()); assertEquals(0, f.store.closes)
            assertEquals(ConnectionUse.SAVED_TRUST, f.access.current())
            assertTrue(replacement.stop())
            assertEquals(ConnectionUse.SAVED_TRUST, f.access.current())
            f.store.closeRelease!!.countDown(); f.settled()
            assertEquals(1, f.store.closes); assertNull(f.access.current())
            assertEquals(SavedTrustPhase.FAILED, f.binding.state.value.phase)
            replacement.close()
        } finally { f.close() }
    }
}
