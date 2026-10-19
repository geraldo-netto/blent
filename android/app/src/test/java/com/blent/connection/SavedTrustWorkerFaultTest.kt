package com.blent.connection

import kotlinx.coroutines.runBlocking
import org.junit.Assert.*
import org.junit.Test

class SavedTrustWorkerFaultTest {
    private fun peer(index: Int) = ConnectionPeer.parse(index.toString(16).padStart(64, '0'))
    private class Store : PeerTrustStore {
        var closed = false
        val original = ConnectionTrustCodec.encode(TrustSnapshot(ConnectionPeer.parse("0".repeat(64)), 1,
            listOf(ConnectionPeer.parse("1".repeat(64)))))
        override fun load() = original.copyOf()
        override fun replace(record: ByteArray) { error("T1151 durable publication failed") }
        override fun close() { closed = true }
    }

    @Test fun t1151StaleAndFailedForgetCannotPublishSuccessOrChangePersistedRecord() = runBlocking<Unit> {
        for (revision in listOf(1L, 2L, Long.MAX_VALUE)) {
            val store = Store()
            val selected = SavedComputerSelection(peer(0), revision, ConnectionPeer.parse("1".repeat(64)))
            var pages = 0
            val worker = SavedTrustWorker(SavedTrustRequest.Forget(selected),
                { SavedTrustAccesses.open(store) { peer(0) } }, { pages++ })
            assertTrue(runCatching { worker.execute() }.isFailure)
            worker.retire()
            assertTrue(store.closed); assertEquals(0, pages)
            assertEquals(1L, ConnectionTrustCodec.decode(store.original).revision)
        }
    }

    @Test fun t1151CancellationAfterFirstPageClosesOriginalAndStopsPublication() = runBlocking<Unit> {
        var closed = false
        val access = object : SavedTrustAccess {
            override fun snapshot() = SavedComputers(peer(0), 1, (1..1024).map(::peer))
            override fun forget(selected: SavedComputerSelection): Boolean = error("T1151 list attempted forgetting")
            override fun close() { closed = true }
        }
        var pages = 0
        lateinit var worker: SavedTrustWorker
        worker = SavedTrustWorker(SavedTrustRequest.List, { access }, { pages++; worker.cancel() })
        worker.execute(); worker.retire()
        assertEquals(1, pages); assertTrue(closed)
        assertTrue(runCatching { worker.execute() }.isFailure)
    }
}
