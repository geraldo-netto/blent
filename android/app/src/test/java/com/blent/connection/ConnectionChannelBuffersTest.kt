package com.blent.connection

import com.blent.connection.ConnectionManagement.Resource
import org.junit.Assert.*
import org.junit.Test
import java.util.Random

class ConnectionChannelBuffersTest {
    private fun rejected(work: () -> Unit) { assertThrows(Exception::class.java) { work() } }
    private fun contents(read: ConnectionChannelRead) = read.bytes.copyOfRange(read.offset, read.offset + read.length)

    @Test fun t882AllResourcesUseFiveIndependentMediaSlots() {
        val buffers = ConnectionChannelBuffers(32)
        val all = Resource.entries.flatMap(ConnectionChannels::members)
        assertEquals(listOf(1, 2, 3, 4, 5), all)
        for (resource in Resource.entries) {
            buffers.install(resource)
            for (channel in ConnectionChannels.members(resource)) {
                assertEquals(resource, ConnectionChannels.resource(channel))
                assertTrue(ConnectionChannels.belongs(resource, channel))
                assertFalse(buffers.readable(channel))
                assertNull(buffers.read(channel, 1))
                buffers.push(channel, byteArrayOf(channel.toByte()))
                assertTrue(buffers.readable(channel))
                val read = buffers.read(channel, 32)!!
                assertArrayEquals(byteArrayOf(channel.toByte()), contents(read))
                assertFalse(buffers.readable(channel))
                assertNull(buffers.read(channel, 1))
                buffers.consumed(channel, read)
                assertFalse(buffers.readable(channel))
            }
            buffers.retire(resource)
            buffers.install(resource)
        }
        assertFalse(ConnectionChannels.belongs(Resource.DISPLAY, 3))
        assertFalse(ConnectionChannels.belongs(Resource.DISPLAY, Int.MAX_VALUE))
        rejected { (ConnectionChannels.members(Resource.DISPLAY) as MutableList<Int>)[0] = 5 }
    }

    @Test fun t882WraparoundRetainsBorrowedCapacityAndNeverMutatesNativeBytes() {
        val buffers = ConnectionChannelBuffers(8)
        buffers.install(Resource.CAMERA)
        buffers.push(3, byteArrayOf(0, 1, 2, 3, 4, 5))
        val first = buffers.read(3, 4)!!
        buffers.push(3, byteArrayOf(6, 7))
        rejected { buffers.push(3, byteArrayOf(8)) }
        assertArrayEquals(byteArrayOf(0, 1, 2, 3), contents(first))
        buffers.consumed(3, first)
        buffers.push(3, byteArrayOf(8, 9, 10, 11))
        val tail = buffers.read(3, 8)!!
        assertSame(first.bytes, tail.bytes)
        assertEquals(4, tail.offset)
        assertArrayEquals(byteArrayOf(4, 5, 6, 7), contents(tail))
        buffers.consumed(3, tail)
        val wrapped = buffers.read(3, 8)!!
        assertSame(first.bytes, wrapped.bytes)
        assertEquals(0, wrapped.offset)
        assertArrayEquals(byteArrayOf(8, 9, 10, 11), contents(wrapped))
        buffers.consumed(3, wrapped)
        assertNull(buffers.read(3, 8))
    }

    @Test fun t882PushCopiesOnlySelectedRangeAcrossRingEnd() {
        val buffers = ConnectionChannelBuffers(5)
        buffers.install(Resource.SPEAKERS)
        buffers.push(5, byteArrayOf(1, 2, 3, 4))
        buffers.consumed(5, buffers.read(5, 4)!!)
        buffers.push(5, byteArrayOf(99, 5, 6, 7, 8, 99), 1, 4)
        val end = buffers.read(5, 5)!!
        assertArrayEquals(byteArrayOf(5), contents(end))
        buffers.consumed(5, end)
        val start = buffers.read(5, 5)!!
        assertArrayEquals(byteArrayOf(6, 7, 8), contents(start))
        buffers.consumed(5, start)
    }

    @Test fun t882SelfAliasedPushRejectsBeforeWraparoundMutatesSource() {
        val buffers = ConnectionChannelBuffers(3)
        buffers.install(Resource.CAMERA)
        buffers.push(3, byteArrayOf(0, 1))
        buffers.consumed(3, buffers.read(3, 1)!!)
        val held = buffers.read(3, 1)!!
        val original = held.bytes.copyOf()
        rejected { buffers.push(3, held.bytes, 1, 2) }
        assertArrayEquals(original, held.bytes)
        assertNull(buffers.read(3, 1))
        buffers.consumed(3, held)
        buffers.push(3, byteArrayOf(2, 3))
        val end = buffers.read(3, 3)!!
        assertArrayEquals(byteArrayOf(2), contents(end))
        buffers.consumed(3, end)
        assertArrayEquals(byteArrayOf(3), contents(buffers.read(3, 3)!!))
    }

    @Test fun t882ForeignAndStaleEqualLengthReceiptsCannotRetireReplacement() {
        val buffers = ConnectionChannelBuffers(8)
        buffers.install(Resource.DISPLAY)
        buffers.push(1, byteArrayOf(1, 2)); buffers.push(2, byteArrayOf(3, 4))
        val first = buffers.read(1, 2)!!; val other = buffers.read(2, 2)!!
        rejected { buffers.consumed(1, other) }
        rejected { buffers.consumed(1, ConnectionChannelRead(first.bytes, first.offset, first.length)) }
        assertNull(buffers.read(1, 2))
        buffers.consumed(1, first)
        buffers.push(1, byteArrayOf(5, 6))
        val second = buffers.read(1, 2)!!
        rejected { buffers.consumed(1, first) }
        assertArrayEquals(byteArrayOf(5, 6), contents(second))
        buffers.consumed(1, second)
        buffers.consumed(2, other)
        rejected { buffers.consumed(1, second) }
    }

    @Test fun t882ActualRetirementDiscardsSlotsWithoutReusingOldBorrowedArrays() {
        val buffers = ConnectionChannelBuffers(8)
        buffers.install(Resource.DISPLAY)
        buffers.push(1, byteArrayOf(1, 2)); buffers.push(2, byteArrayOf(3))
        val old = buffers.read(1, 2)!!
        // Caller has established native consumer exit; no fabricated timeout here.
        buffers.retire(Resource.DISPLAY)
        rejected { buffers.consumed(1, old) }
        rejected { buffers.read(2, 1) }
        buffers.retire(Resource.DISPLAY)
        buffers.install(Resource.DISPLAY)
        buffers.push(1, byteArrayOf(4, 5))
        val fresh = buffers.read(1, 2)!!
        assertNotSame(old.bytes, fresh.bytes)
        rejected { buffers.consumed(1, old) }
        assertArrayEquals(byteArrayOf(1, 2), contents(old))
        buffers.consumed(1, fresh)
    }

    @Test fun t882InvalidCapacitiesRangesAndChannelsDoNotMutateValidState() {
        for (capacity in listOf(Int.MIN_VALUE, -1, 0, ConnectionWire.MAX_CREDIT + 1, Int.MAX_VALUE)) rejected { ConnectionChannelBuffers(capacity) }
        for (capacity in listOf(1, 2, 4096, ConnectionWire.MAX_CREDIT)) {
            val buffers = ConnectionChannelBuffers(capacity)
            buffers.install(Resource.MICROPHONE)
            buffers.push(4, byteArrayOf(7))
            assertEquals(1, buffers.read(4, 1)!!.length)
        }
        val buffers = ConnectionChannelBuffers(8)
        invalidChannels(buffers)
        rejected { buffers.push(3, byteArrayOf(1)) }
        buffers.install(Resource.CAMERA)
        rejected { buffers.install(Resource.CAMERA) }
        invalidRanges(buffers)
        buffers.push(3, byteArrayOf(7))
        assertArrayEquals(byteArrayOf(7), contents(buffers.read(3, 1)!!))
    }
    private fun invalidChannels(buffers: ConnectionChannelBuffers) {
        for (channel in listOf(Int.MIN_VALUE, -1, 0, 6, Int.MAX_VALUE)) {
            rejected { ConnectionChannels.resource(channel) }
            rejected { buffers.readable(channel) }
            rejected { buffers.read(channel, 1) }
            rejected { buffers.push(channel, byteArrayOf(1)) }
        }
    }
    private fun invalidRanges(buffers: ConnectionChannelBuffers) {
        for ((offset, length) in listOf(-1 to 1, 0 to -1, 5 to 0, 3 to 2, Int.MAX_VALUE to Int.MAX_VALUE)) {
            rejected { buffers.push(3, ByteArray(4), offset, length) }
        }
        buffers.push(3, ByteArray(4), 4, 0)
        for (size in listOf(Int.MIN_VALUE, 0, ConnectionWire.MAX_PAYLOAD + 1, Int.MAX_VALUE)) rejected { buffers.read(3, size) }
    }

    @Test fun t882BoundedRandomOperationsMatchQueueAndPreserveOutstandingLoan() {
        for (capacity in listOf(1, 7, 32, 127)) randomRing(capacity)
    }
    private fun randomRing(capacity: Int) {
        val buffers = ConnectionChannelBuffers(capacity)
        buffers.install(Resource.MICROPHONE)
        val expected = java.util.ArrayDeque<Byte>()
        val random = Random(882L + capacity)
        var loan: ConnectionChannelRead? = null
        repeat(4096) {
            when (random.nextInt(3)) {
                0 -> randomPush(buffers, expected, random, capacity)
                1 -> { if (loan == null) loan = buffers.read(4, 1 + random.nextInt(65536)) else assertNull(buffers.read(4, 1)) }
                else -> { loan?.let { read -> checkBorrow(read, expected); buffers.consumed(4, read); repeat(read.length) { expected.removeFirst() } }; loan = null }
            }
            loan?.let { checkBorrow(it, expected) }
            assertEquals(expected.isNotEmpty() && loan == null, buffers.readable(4))
        }
    }
    private fun randomPush(buffers: ConnectionChannelBuffers, expected: java.util.ArrayDeque<Byte>, random: Random, capacity: Int) {
        val bytes = ByteArray(random.nextInt(capacity + 2)).apply { random.nextBytes(this) }
        if (bytes.size > capacity - expected.size) rejected { buffers.push(4, bytes) }
        else { buffers.push(4, bytes); bytes.forEach(expected::addLast) }
    }
    private fun checkBorrow(read: ConnectionChannelRead, expected: java.util.ArrayDeque<Byte>) {
        assertArrayEquals(expected.take(read.length).toByteArray(), contents(read))
    }
}
