package com.blent.connection

import org.junit.Assert.*
import org.junit.Test

class ConnectionQueueTest {
    @Test fun t846FairQueueSkipsStallsWithoutCopying() {
        val queue = ConnectionQueue(64, 4, 4)
        val bytes = byteArrayOf(1, 2, 3, 4, 5, 6)
        assertTrue(queue.offer(1, bytes))
        assertTrue(queue.offer(2, ByteArray(16) { 9 }))
        assertTrue(queue.offer(4, ByteArray(3) { 7 }))
        val credit = intArrayOf(64, 0, 0, 64, 0, 0)
        val first = queue.next(credit)!!
        assertSame(bytes, first.bytes)
        assertEquals(1, first.channel); assertEquals(4, first.length); assertEquals(0, first.offset)
        assertNull(queue.next(IntArray(ConnectionWire.CHANNELS) { 64 }))
        assertThrows(IllegalStateException::class.java) { queue.clear(1) }
        queue.clear(3); queue.finish(true)
        assertEquals(4, queue.next(credit)!!.channel); queue.finish(false)
        val last = queue.next(credit)!!
        assertEquals(1, last.channel); assertEquals(4, last.offset); assertEquals(2, last.length)
        queue.finish(true)
        assertEquals(4, queue.next(credit)!!.channel); queue.finish(true)
        assertNull(queue.next(IntArray(ConnectionWire.CHANNELS)))
        assertEquals(1, queue.next(intArrayOf(0, 1, 0, 0, 0, 0))!!.length); queue.finish(true)
        queue.clear(2); assertNull(queue.next(IntArray(ConnectionWire.CHANNELS) { 64 }))
        assertThrows(IllegalStateException::class.java) { queue.finish(true) }
    }
    @Test fun t846QueueBoundsRetainWholeAllocationUntilConsumed() {
        for (value in listOf(Int.MIN_VALUE, -1, 0, Int.MAX_VALUE)) {
            assertThrows(IllegalArgumentException::class.java) { ConnectionQueue(value, 1, 1) }
            assertThrows(IllegalArgumentException::class.java) { ConnectionQueue(8, value, 1) }
            assertThrows(IllegalArgumentException::class.java) { ConnectionQueue(8, 1, value) }
        }
        val queue = ConnectionQueue(8, 1, 4)
        assertFalse(queue.offer(1, byteArrayOf()))
        assertFalse(queue.offer(1, ByteArray(ConnectionWire.MAX_PAYLOAD + 1)))
        assertFalse(queue.offer(1, ByteArray(9)))
        assertTrue(queue.offer(1, ByteArray(8)))
        queue.next(IntArray(ConnectionWire.CHANNELS) { 8 }); queue.finish(true)
        assertFalse(queue.offer(1, byteArrayOf(1)))
        queue.next(IntArray(ConnectionWire.CHANNELS) { 8 }); queue.finish(true)
        assertTrue(queue.offer(1, byteArrayOf(1)))
        assertFalse(queue.offer(1, byteArrayOf(1)))
        for (channel in listOf(-1, 0, ConnectionWire.CHANNELS + 1, Int.MAX_VALUE)) {
            assertThrows(IllegalArgumentException::class.java) { queue.offer(channel, byteArrayOf(1)) }
            assertThrows(IllegalArgumentException::class.java) { queue.clear(channel) }
        }
        assertThrows(IllegalArgumentException::class.java) { queue.next(IntArray(ConnectionWire.CHANNELS + 1)) }
        assertThrows(IllegalArgumentException::class.java) { queue.next(intArrayOf(1, 1, -1, 1, 1)) }
        assertThrows(IllegalArgumentException::class.java) { queue.next(IntArray(ConnectionWire.CHANNELS) { Int.MAX_VALUE }) }
    }
    @Test fun t846SeededQueuePreservesOrderWithCancellation() {
        val queue = ConnectionQueue(64, 8, 4)
        val expected = Array(ConnectionWire.CHANNELS) { java.util.ArrayDeque<Byte>() }
        val random = java.util.Random(846)
        repeat(4096) {
            val index = random.nextInt(ConnectionWire.CHANNELS)
            val payload = ByteArray(random.nextInt(80)) { random.nextInt(256).toByte() }
            if (queue.offer(index + 1, payload)) payload.forEach { expected[index].addLast(it) }
            val credit = IntArray(ConnectionWire.CHANNELS) { if (it == index) 0 else random.nextInt(8) + 1 }
            val slice = queue.next(credit)
            if (slice != null) {
                assertTrue(slice.length <= 4 && slice.length <= credit[slice.channel - 1])
                verify(expected[slice.channel - 1], slice)
                val sent = random.nextBoolean()
                if (sent) repeat(slice.length) { expected[slice.channel - 1].removeFirst() }
                queue.finish(sent)
            }
        }
        while (true) {
            val slice = queue.next(IntArray(ConnectionWire.CHANNELS) { 64 }) ?: break
            verify(expected[slice.channel - 1], slice)
            repeat(slice.length) { expected[slice.channel - 1].removeFirst() }
            queue.finish(true)
        }
        assertTrue(expected.all { it.isEmpty() })
    }
    private fun verify(expected: java.util.ArrayDeque<Byte>, slice: ConnectionQueue.Slice) {
        assertEquals(expected.take(slice.length), slice.bytes.slice(slice.offset until slice.offset + slice.length))
    }
}
