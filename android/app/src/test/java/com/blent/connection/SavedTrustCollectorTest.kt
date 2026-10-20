package com.blent.connection

import org.junit.Assert.*
import org.junit.Test

internal fun savedPeer(index: Int) = ConnectionPeer.parse(index.toString(16).padStart(64, '0'))
internal fun savedPages(total: Int, revision: Long = 1): List<SavedComputersPage> {
    val peers = (1..total).map(::savedPeer)
    if (total == 0) return listOf(SavedComputersPage(savedPeer(0), revision, 0, 0, emptyList()))
    return peers.chunked(48).mapIndexed { index, values -> SavedComputersPage(savedPeer(0), revision, total, index * 48, values) }
}

class SavedTrustCollectorTest {
    @Test fun t1168EveryAdmittedTotalProducesOnlyOneCompleteBoundedResult() {
        for (total in 0..1024) {
            val collector = SavedTrustCollector()
            assertThrows(IllegalStateException::class.java) { collector.result() }
            val pages = savedPages(total)
            pages.forEachIndexed { index, page ->
                collector.add(page)
                if (index < pages.lastIndex) assertThrows(IllegalStateException::class.java) { collector.result() }
            }
            val result = checkNotNull(collector.result().computers)
            assertEquals(total, result.peers.size); assertEquals(savedPeer(0), result.local); assertEquals(1L, result.revision)
            assertThrows(IllegalStateException::class.java) { collector.add(pages.last()) }
        }
    }
    @Test fun t1168NoIdentityIsACompleteEmptyResultRatherThanMissingPages() {
        val collector = SavedTrustCollector()
        collector.add(SavedComputersPage(null, 0, 0, 0, emptyList()))
        assertNull(collector.result().computers)
        assertThrows(IllegalStateException::class.java) { collector.add(SavedComputersPage(null, 0, 0, 0, emptyList())) }
        val empty = SavedTrustCollector()
        savedPages(0, 0).forEach(empty::add)
        assertEquals(0L, empty.result().computers!!.revision)
    }
    @Test fun t1168PageGapsReplaysHeaderChangesAndCrossPageOverlapReject() {
        val pages = savedPages(96)
        assertThrows(IllegalArgumentException::class.java) { SavedTrustCollector().add(pages[1]) }
        val malformed = listOf(
            pages[0],
            SavedComputersPage(savedPeer(999), 1, 96, 48, pages[1].peers),
            SavedComputersPage(savedPeer(0), 2, 96, 48, pages[1].peers),
            SavedComputersPage(savedPeer(0), 1, 97, 48, pages[1].peers),
            SavedComputersPage(savedPeer(0), 1, 96, 48, (1..48).map(::savedPeer)),
            SavedComputersPage(savedPeer(0), 1, 96, 48, (48..95).map(::savedPeer)),
        )
        for (bad in malformed) {
            val collector = SavedTrustCollector(); collector.add(pages[0])
            assertThrows(IllegalArgumentException::class.java) { collector.add(bad) }
            assertThrows(IllegalStateException::class.java) { collector.result() }
            collector.add(pages[1]); assertEquals(96, collector.result().computers!!.peers.size)
        }
    }
}
