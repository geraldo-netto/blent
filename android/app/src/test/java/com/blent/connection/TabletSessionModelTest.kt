package com.blent.connection

import com.blent.connection.ConnectionManagement.Resource
import org.junit.Assert.*
import org.junit.Test
import java.util.Random

class TabletSessionModelTest {
    private fun consume(rig: TabletSessionRig, lease: ConnectionResourceLease, channel: Int, expected: ByteArray) {
        var offset = 0
        while (offset < expected.size) {
            val read = checkNotNull(rig.tablet.read(lease, channel, 31))
            assertArrayEquals(expected.copyOfRange(offset, offset + read.length),
                read.bytes.copyOfRange(read.offset, read.offset + read.length))
            rig.tablet.consumed(lease, channel, read)
            assertThrows(IllegalArgumentException::class.java) { rig.tablet.consumed(lease, channel, read) }
            offset += read.length
        }
        assertNull(rig.tablet.read(lease, channel, 31))
    }
    @Test fun t882BoundedDuplexModelPreservesAllResourceBytesAcrossWraps() {
        val rig = TabletSessionRig(quantum = 17)
        val leases = Resource.entries.associateWith { rig.admit(it, it.ordinal + 1L) }
        val random = Random(882)
        repeat(512) {
            val channel = random.nextInt(5) + 1
            val lease = checkNotNull(leases[ConnectionChannels.resource(channel)])
            val bytes = ByteArray(random.nextInt(127) + 1).also(random::nextBytes)
            assertTrue(rig.host.offer(channel, bytes)); rig.pump()
            consume(rig, lease, channel, bytes)
            assertTrue(rig.tablet.offer(lease, channel, bytes.copyOf())); rig.pump()
            assertArrayEquals(bytes, rig.payloads.flatMap { it.second.asIterable() }.toByteArray())
            assertTrue(rig.payloads.all { it.first == channel }); rig.payloads.clear(); rig.events()
        }
    }
    @Test fun t882ReadinessCoalescesAndRotatesAcrossResourceLanes() {
        val rig = TabletSessionRig()
        val leases = Resource.entries.associateWith { rig.admit(it, it.ordinal + 1L) }
        for (channel in 1..5) { assertTrue(rig.host.offer(channel, byteArrayOf(channel.toByte()))) }
        rig.pump()
        val events = rig.events().map { it as ConnectionSessionEvent.Readable }
        assertEquals((1..5).toList(), events.map { it.channel })
        for (event in events) {
            assertEquals(leases[ConnectionChannels.resource(event.channel)], event.lease)
            consume(rig, event.lease, event.channel, byteArrayOf(event.channel.toByte()))
        }
        rig.pump(); assertTrue(rig.events().isEmpty())
    }
}
