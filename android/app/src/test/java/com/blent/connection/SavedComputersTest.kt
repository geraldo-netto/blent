package com.blent.connection

import org.junit.Assert.*
import org.junit.Test

class SavedComputersTest {
    private fun peer(index: Int) = ConnectionPeer.parse(index.toString(16).padStart(64, '0'))
    private class Store : PeerTrustStore {
        var bytes: ByteArray? = null
        var fail = false
        var writes = 0
        var closed = 0
        override fun load(): ByteArray? = bytes?.copyOf()
        override fun replace(record: ByteArray) {
            check(!fail)
            bytes = record.copyOf(); writes++
        }
        override fun close() { closed++ }
    }
    private fun remember(trust: ConnectionTrust, pairing: ConnectionPairing, index: Int): ConnectionApproval {
        val ticket = trust.begin(pairing, peer(index), 0, 100)
        pairing.approve(ticket, true, 1)
        return trust.authenticated(pairing, ticket, peer(index), 2)
    }
    private fun rejected(body: () -> Unit) { assertThrows(Exception::class.java, body) }

    @Test fun t1150EmptySnapshotDoesNotPublishOrGrantAuthority() {
        val store = Store()
        val trust = ConnectionTrust(store, peer(0))
        val view = trust.snapshot()
        assertEquals(peer(0), view.local)
        assertEquals(0L, view.revision)
        assertTrue(view.peers.isEmpty())
        assertEquals(0, store.writes)
        assertNull(store.bytes)
        rejected { view.select(peer(1)) }
        assertFalse(trust.trusted(peer(1)))
        trust.close()
        assertEquals(1, store.closed)
        rejected { trust.snapshot() }
    }
    @Test fun t1150SnapshotsAndSelectionAreDetachedAndBounded() {
        val original = mutableListOf(peer(1), peer(2))
        val view = SavedComputers(peer(0), Long.MAX_VALUE, original)
        original.clear()
        val exposed = view.peers as MutableList<ConnectionPeer>
        exposed.clear()
        assertEquals(listOf(peer(1), peer(2)), view.peers)
        assertEquals(SavedComputerSelection(peer(0), Long.MAX_VALUE, peer(2)), view.select(peer(2)))
        rejected { view.select(peer(3)) }
        for (revision in listOf(Long.MIN_VALUE, -1, 0, 1, Long.MAX_VALUE)) {
            assertEquals(revision >= 0, runCatching { SavedComputers(peer(0), revision, emptyList()) }.isSuccess)
            assertEquals(revision > 0, runCatching { SavedComputerSelection(peer(0), revision, peer(1)) }.isSuccess)
        }
        rejected { SavedComputers(peer(0), 0, listOf(peer(1))) }
        rejected { SavedComputers(peer(0), 1, listOf(peer(2), peer(1))) }
        rejected { SavedComputers(peer(0), 1, listOf(peer(1), peer(1))) }
        for (count in listOf(0, 1, 64, 1024, 1025)) {
            val peers = (1..count).map(::peer)
            assertEquals(count <= 1024, runCatching { SavedComputers(peer(0), 1, peers) }.isSuccess)
        }
    }
    @Test fun t1150ConditionalForgetRejectsStaleAndForeignListsBeforeLiveRevoke() {
        val store = Store()
        val trust = ConnectionTrust(store, peer(0))
        val first = ConnectionPairing()
        val original = remember(trust, first, 1)
        val stale = trust.snapshot().select(peer(1))
        val second = ConnectionPairing()
        val active = remember(trust, second, 2)
        val before = store.bytes!!.copyOf()
        rejected { trust.forget(second, stale) }
        rejected { trust.forget(second, SavedComputerSelection(peer(9), 2, peer(2))) }
        assertTrue(first.current(original))
        assertTrue(second.current(active))
        assertArrayEquals(before, store.bytes)
        assertFalse(trust.forget(second, SavedComputerSelection(peer(0), 2, peer(9))))
        assertTrue(second.current(active))
        val view = trust.snapshot()
        assertTrue(trust.forget(second, view.select(peer(2))))
        assertFalse(second.current(active))
        assertTrue(first.current(original))
        assertEquals(listOf(peer(1)), trust.snapshot().peers)
        assertEquals(3L, trust.snapshot().revision)
        rejected { trust.forget(first, view.select(peer(1))) }
        assertTrue(first.current(original))
        trust.close()
        val restored = ConnectionTrust(store, peer(0))
        assertEquals(3L, restored.snapshot().revision)
        assertEquals(listOf(peer(1)), restored.snapshot().peers)
        restored.close()
    }
    @Test fun t1150StorageFailureAndExhaustionCannotExposeSuccessfulForget() {
        val store = Store()
        val trust = ConnectionTrust(store, peer(0))
        val pairing = ConnectionPairing()
        val active = remember(trust, pairing, 1)
        val selected = trust.snapshot().select(peer(1))
        val durable = store.bytes!!.copyOf()
        store.fail = true
        rejected { trust.forget(pairing, selected) }
        assertFalse(pairing.current(active))
        assertFalse(trust.trusted(peer(1)))
        assertArrayEquals(durable, store.bytes)
        rejected { trust.snapshot() }
        store.fail = false
        rejected { trust.forget(pairing, selected) }
        trust.close()
        rejected { trust.forget(pairing, selected) }
        val exhaustedStore = Store().apply {
            bytes = ConnectionTrustCodec.encode(TrustSnapshot(peer(0), Long.MAX_VALUE, listOf(peer(1))))
        }
        val exhausted = ConnectionTrust(exhaustedStore, peer(0))
        val view = exhausted.snapshot()
        rejected { exhausted.forget(ConnectionPairing(), view.select(peer(1))) }
        assertEquals(0, exhaustedStore.writes)
        rejected { exhausted.snapshot() }
        exhausted.close()
    }
    @Test fun t1150BoundedStaleRevisionAndUnknownPeerCorpusNeverMutatesTrust() {
        val store = Store()
        val trust = ConnectionTrust(store, peer(0))
        val pairing = ConnectionPairing()
        val active = remember(trust, pairing, 1)
        val record = store.bytes!!.copyOf()
        for (revision in 2L..1025L) {
            rejected { trust.forget(pairing, SavedComputerSelection(peer(0), revision, peer(1))) }
        }
        for (index in 2..1025) {
            assertFalse(trust.forget(pairing, SavedComputerSelection(peer(0), 1, peer(index))))
        }
        assertArrayEquals(record, store.bytes)
        assertEquals(1, store.writes)
        assertTrue(pairing.current(active))
        assertEquals(listOf(peer(1)), trust.snapshot().peers)
        trust.close()
    }
}
