package com.blent.connection

import java.nio.ByteBuffer
import org.junit.Assert.*
import org.junit.Test

class SavedTrustProtocolTest {
    private fun peer(index: Int) = ConnectionPeer.parse(index.toString(16).padStart(64, '0'))
    private fun rejected(body: () -> Unit) { assertThrows(Exception::class.java, body) }
    @Test fun t1151RequestCodecKeepsExactBoundedConditionalIdentity() {
        assertArrayEquals(byteArrayOf(1, 0), SavedTrustCodec.request(SavedTrustRequest.List))
        val selected = SavedComputerSelection(peer(0), Long.MAX_VALUE, peer(1))
        val request = SavedTrustRequest.Forget(selected)
        val bytes = SavedTrustCodec.request(request)
        assertEquals(142, bytes.size)
        assertEquals(request, SavedTrustCodec.request(bytes))
        assertEquals(SavedTrustRequest.List, SavedTrustCodec.request(byteArrayOf(1, 0)))
        for (count in 0 until bytes.size) rejected { SavedTrustCodec.request(bytes.copyOf(count)) }
        rejected { SavedTrustCodec.request(bytes + byteArrayOf(0)) }
        rejected { SavedTrustCodec.request(byteArrayOf(2, 0)) }
        for (kind in 2..255) rejected { SavedTrustCodec.request(byteArrayOf(1, kind.toByte())) }
        rejected { SavedTrustCodec.request(byteArrayOf(1, 0, 0)) }
        val zero = bytes.copyOf()
        ByteBuffer.wrap(zero).putLong(68, 0)
        rejected { SavedTrustCodec.request(zero) }
    }
    @Test fun t1151PagesRoundTripAllBoundariesWithinExistingPacketAndQueueLimits() {
        for (count in listOf(0, 1, 47, 48, 49, 1024)) {
            val all = (1..count).map(::peer)
            val offsets = if (count == 0) listOf(0) else all.indices.step(48).toList()
            val restored = mutableListOf<ConnectionPeer>()
            for (offset in offsets) {
                val page = SavedComputersPage(peer(0), 1, count, offset, all.drop(offset).take(48))
                val wire = ConnectionWorkerCodec.event(ConnectionWorkerEvent.SavedComputers(page))
                assertTrue(wire.size <= ConnectionWorkerCodec.LIMIT)
                val parsed = (ConnectionWorkerCodec.event(wire) as ConnectionWorkerEvent.SavedComputers).page
                assertEquals(peer(0), parsed.local); assertEquals(1L, parsed.revision)
                assertEquals(count, parsed.total); assertEquals(offset, parsed.offset)
                assertEquals(page.peers, parsed.peers)
                restored.addAll(parsed.peers)
            }
            assertEquals(all, restored)
            assertTrue(offsets.size + 1 < 32)
        }
        val fresh = SavedComputersPage(null, 0, 0, 0, emptyList())
        val parsed = ConnectionWorkerCodec.event(ConnectionWorkerCodec.event(ConnectionWorkerEvent.SavedComputers(fresh)))
            as ConnectionWorkerEvent.SavedComputers
        assertNull(parsed.page.local); assertEquals(0, parsed.page.total)
    }
    @Test fun t1151PageFieldsRejectInvalidCountsRangesIdentitiesAndTruncations() {
        val original = ConnectionWorkerCodec.event(ConnectionWorkerEvent.SavedComputers(
            SavedComputersPage(peer(0), 1, 1, 0, listOf(peer(1))),
        ))
        for (count in 0 until original.size) rejected { ConnectionWorkerCodec.event(original.copyOf(count)) }
        rejected { ConnectionWorkerCodec.event(original + byteArrayOf(0)) }
        for (value in listOf(Int.MIN_VALUE, -1, 49, 1025, Int.MAX_VALUE)) {
            val bytes = original.copyOf(); ByteBuffer.wrap(bytes).putInt(85, value)
            rejected { ConnectionWorkerCodec.event(bytes) }
        }
        for (total in listOf(-1, 0, 2, 1025)) {
            val bytes = original.copyOf(); ByteBuffer.wrap(bytes).putInt(77, total)
            rejected { ConnectionWorkerCodec.event(bytes) }
        }
        val bad = original.copyOf(); bad[2] = 2
        rejected { ConnectionWorkerCodec.event(bad) }
        rejected { SavedComputersPage(null, 1, 0, 0, emptyList()) }
        rejected { SavedComputersPage(peer(0), 0, 1, 0, listOf(peer(1))) }
        rejected { SavedComputersPage(peer(0), -1, 0, 0, emptyList()) }
        rejected { SavedComputersPage(peer(0), 1, 1, 1, emptyList()) }
        rejected { SavedComputersPage(peer(0), 1, 2, 0, listOf(peer(2), peer(1))) }
        rejected { SavedComputersPage(peer(0), 1, 2, 0, listOf(peer(1), peer(1))) }
    }
    @Test fun t1151PageRejectsUnadmittedElementsBeforeCopy() {
        var reads = 0
        val values = object : AbstractList<ConnectionPeer>() {
            override val size: Int get() = 49
            override fun get(index: Int): ConnectionPeer { reads++; error("unadmitted peer") }
        }
        assertThrows(IllegalArgumentException::class.java) { SavedComputersPage(peer(0), 1, 49, 0, values) }
        assertEquals(0, reads)
    }
}
