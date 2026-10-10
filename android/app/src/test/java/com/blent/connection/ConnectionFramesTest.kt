package com.blent.connection

import java.io.ByteArrayOutputStream
import org.junit.Assert.*
import org.junit.Test

class ConnectionFramesTest {
    private fun data(size: Int = 9, channel: Int = 2) = ConnectionFrame(
        ConnectionWire.Header(ConnectionWire.DATA, channel, 1, 1, 0, size, 0),
        ByteArray(size + 5) { (it * 31).toByte() }, 3, size,
    )
    private fun rejects(block: () -> Unit) { assertTrue(runCatching(block).isFailure) }

    private fun encode(frame: ConnectionFrame, chunk: Int): ByteArray {
        val writer = ConnectionFrameWriter(1000)
        writer.begin(frame, 0)
        val output = ByteArrayOutputStream()
        while (true) {
            val loan = writer.write(1) ?: break
            assertNull(writer.write(1))
            assertNull(writer.flush(1))
            val count = minOf(chunk, loan.length)
            output.write(loan.bytes, loan.offset, count)
            writer.writeComplete(loan, count, 1)
        }
        val flush = checkNotNull(writer.flush(1))
        assertNull(writer.flush(1))
        assertNull(writer.write(1))
        assertTrue(writer.nativePending())
        assertSame(frame, writer.flushComplete(flush, 1))
        assertFalse(writer.nativePending())
        rejects { writer.flushComplete(flush, 1) }
        return output.toByteArray()
    }
    private fun deliver(reader: ConnectionFrameReader, bytes: ByteArray, chunk: Int) {
        var offset = 0
        while (offset < bytes.size) {
            val loan = checkNotNull(reader.read(1))
            assertNull(reader.read(1))
            assertTrue(reader.nativePending())
            val count = minOf(chunk, loan.length, bytes.size - offset)
            bytes.copyInto(loan.bytes, loan.offset, offset, offset + count)
            reader.readComplete(loan, count, 1)
            offset += count
        }
        assertFalse(reader.nativePending())
    }

    @Test fun t881CancelledCompletedFrameCannotBeDispatched() {
        val reader = ConnectionFrameReader(1000)
        deliver(reader, encode(data(), 4), 7)
        reader.cancel()
        assertNull("Cancelled frame escaped transport gate", reader.frame())
    }
    @Test fun t881CompletedFrameHasOneConsumerClaim() {
        val reader = ConnectionFrameReader(1000)
        deliver(reader, encode(data(), 40), 40)
        val frame = checkNotNull(reader.frame())
        assertNull("Same frame could be dispatched twice", reader.frame())
        assertNull(reader.read(1))
        reader.cancel()
        reader.release(frame)
        rejects { reader.release(frame) }
        rejects { reader.read(2) }
    }
    @Test fun t881FragmentedRoundTripsAllChannelsAndMaximumExtent() {
        for (channel in 1..ConnectionWire.CHANNELS) {
            for (size in listOf(1, 4096, ConnectionWire.MAX_PAYLOAD)) {
                val frame = data(size, channel)
                val reader = ConnectionFrameReader(1000)
                deliver(reader, encode(frame, 37), 29)
                val received = checkNotNull(reader.frame())
                assertEquals(frame.header, received.header)
                assertArrayEquals(frame.bytes.copyOfRange(3, size + 3), received.bytes.copyOf(received.length))
                reader.release(received)
                assertNotNull(reader.read(2))
            }
        }
    }
    @Test fun t881MetadataFramesNeedNoPayloadReadAndReaderCanBeReused() {
        val reader = ConnectionFrameReader(1000)
        for (kind in ConnectionWire.CREDIT..ConnectionWire.RESET) {
            val credit = if (kind == ConnectionWire.CREDIT) 4 else 0
            val header = ConnectionWire.Header(kind, 1, 1, 1, 0, 0, credit)
            deliver(reader, encode(ConnectionFrame(header, byteArrayOf(), 0, 0), 1), 1)
            val received = checkNotNull(reader.frame())
            assertEquals(header, received.header)
            assertEquals(0, received.length)
            assertNull(reader.read(1))
            reader.release(received)
        }
    }
    @Test fun t881ReadIdleZeroProgressAndPartialDeadlineHaveDistinctBudgets() {
        val reader = ConnectionFrameReader(10)
        val idle = checkNotNull(reader.read(0))
        reader.tick(100)
        reader.readComplete(idle, 0, 100)
        val start = checkNotNull(reader.read(100))
        start.bytes[0] = 0x42
        reader.readComplete(start, 1, 100)
        val stalled = checkNotNull(reader.read(101))
        reader.tick(109)
        rejects { reader.tick(110) }
        assertTrue(reader.nativePending())
        rejects { reader.readComplete(stalled, 1, 110) }
        assertFalse(reader.nativePending())
        rejects { reader.read(110) }
    }
    @Test fun t881WriteDeadlineIncludesFlushAndKeepsNativeBorrow() {
        val writer = ConnectionFrameWriter(10)
        val frame = data(1)
        writer.begin(frame, 0)
        while (true) {
            val loan = writer.write(1) ?: break
            writer.writeComplete(loan, loan.length, 1)
        }
        val flush = checkNotNull(writer.flush(1))
        writer.tick(9)
        rejects { writer.tick(10) }
        assertTrue(writer.nativePending())
        rejects { writer.flushComplete(flush, 10) }
        assertFalse(writer.nativePending())
        rejects { writer.begin(frame, 11) }
    }
    @Test fun t881CancelledNativeWriteCannotAdvanceOrReplaceItsFrame() {
        val writer = ConnectionFrameWriter(1000)
        writer.begin(data(), 0)
        val loan = checkNotNull(writer.write(0))
        val original = loan.bytes.copyOf()
        writer.cancel()
        rejects { writer.begin(data(100), 1) }
        assertArrayEquals(original, loan.bytes)
        assertTrue(writer.nativePending())
        rejects { writer.writeComplete(loan, 1, 1) }
        assertFalse(writer.nativePending())
        rejects { writer.write(1) }
    }
    @Test fun t881StaleNativeCallbacksNeverConsumeTheCurrentLoan() {
        val reader = ConnectionFrameReader(1000)
        val oldRead = checkNotNull(reader.read(0))
        reader.readComplete(oldRead, 0, 0)
        val newRead = checkNotNull(reader.read(0))
        rejects { reader.readComplete(oldRead, 1, 0) }
        assertTrue(reader.nativePending())
        reader.readComplete(newRead, 0, 0)
        val writer = ConnectionFrameWriter(1000)
        writer.begin(data(), 0)
        val oldWrite = checkNotNull(writer.write(0))
        writer.writeComplete(oldWrite, 0, 0)
        val newWrite = checkNotNull(writer.write(0))
        rejects { writer.writeComplete(oldWrite, 1, 0) }
        assertTrue(writer.nativePending())
        writer.writeComplete(newWrite, 1, 0)
    }
    @Test fun t881InvalidNativeExtentsPoisonOnlyTheirExactTransport() {
        for (count in listOf(Int.MIN_VALUE, -1, 41, Int.MAX_VALUE)) {
            val reader = ConnectionFrameReader(1000)
            val loan = checkNotNull(reader.read(0))
            rejects { reader.readComplete(loan, count, 0) }
            assertFalse(reader.nativePending())
            rejects { reader.read(0) }
            val writer = ConnectionFrameWriter(1000)
            writer.begin(data(), 0)
            val output = checkNotNull(writer.write(0))
            rejects { writer.writeComplete(output, count, 0) }
            assertFalse(writer.nativePending())
            rejects { writer.write(0) }
        }
    }
    @Test fun t881InvalidFrameBoundsRejectBeforeNativeWrite() {
        val writer = ConnectionFrameWriter(1000)
        val valid = data()
        for (offset in listOf(Int.MIN_VALUE, -1, 6, Int.MAX_VALUE)) {
            rejects { writer.begin(valid.copy(offset = offset), 0) }
            assertNull(writer.write(0))
        }
        for (length in listOf(Int.MIN_VALUE, -1, 0, 8, 10, Int.MAX_VALUE)) {
            rejects { writer.begin(valid.copy(length = length), 0) }
        }
        writer.begin(valid, 0)
        rejects { writer.begin(valid, 0) }
        assertNotNull(writer.write(0))
    }
    @Test fun t881MalformedHeaderAndInvalidClocksFailClosed() {
        val reader = ConnectionFrameReader(1000)
        val loan = checkNotNull(reader.read(0))
        loan.bytes.fill(0)
        rejects { reader.readComplete(loan, loan.length, 0) }
        rejects { reader.read(0) }
        val backwards = ConnectionFrameReader(1000)
        val pending = checkNotNull(backwards.read(5))
        rejects { backwards.readComplete(pending, 0, 4) }
        assertFalse(backwards.nativePending())
        for (budget in listOf(Long.MIN_VALUE, -1L, 0L, 60001L, Long.MAX_VALUE)) {
            rejects { ConnectionFrameReader(budget) }
            rejects { ConnectionFrameWriter(budget) }
        }
        rejects { ConnectionFrameWriter(1000).begin(data(), Long.MAX_VALUE) }
    }

    @Test fun t881CoalescedFramesStopAtEachConsumerBoundary() {
        val expected = (1..6).map { data(it * 7, it) }
        val bytes = expected.flatMap { encode(it, 13).asList() }.toByteArray()
        val reader = ConnectionFrameReader(1000)
        var offset = 0
        for (original in expected) {
            var received: ConnectionFrame? = null
            while (received == null) {
                val loan = checkNotNull(reader.read(1))
                val count = minOf(loan.length, bytes.size - offset)
                bytes.copyInto(loan.bytes, loan.offset, offset, offset + count)
                reader.readComplete(loan, count, 1)
                offset += count
                received = reader.frame()
            }
            assertEquals(original.header, received.header)
            assertArrayEquals(original.bytes.copyOfRange(3, 3 + original.length), received.bytes.copyOf(received.length))
            assertNull(reader.read(1))
            reader.release(received)
        }
        assertEquals(bytes.size, offset)
    }

    @Test fun t881WriterBorrowsOriginalPayloadAndCannotReuseBeforeFlush() {
        val writer = ConnectionFrameWriter(1000)
        for (size in listOf(1, 10, ConnectionWire.MAX_PAYLOAD)) {
            val frame = data(size)
            writer.begin(frame, 0)
            val header = checkNotNull(writer.write(0))
            writer.writeComplete(header, header.length, 0)
            val body = checkNotNull(writer.write(0))
            assertSame(frame.bytes, body.bytes)
            assertEquals(frame.offset, body.offset)
            assertEquals(frame.length, body.length)
            writer.writeComplete(body, body.length, 0)
            rejects { writer.begin(data(), 0) }
            val flush = checkNotNull(writer.flush(0))
            assertSame(frame, writer.flushComplete(flush, 0))
        }
        assertNull(writer.write(1))
        assertNull(writer.flush(1))
    }

    @Test fun t881BoundedHeaderMutationsNeverEscapeBufferExtents() {
        val encoded = encode(data(1), 40)
        val random = java.util.Random(881)
        repeat(512) {
            val reader = ConnectionFrameReader(1000)
            val loan = checkNotNull(reader.read(0))
            encoded.copyInto(loan.bytes, 0, 0, ConnectionWire.HEADER_BYTES)
            val index = random.nextInt(ConnectionWire.HEADER_BYTES)
            loan.bytes[index] = random.nextInt(256).toByte()
            if (runCatching { reader.readComplete(loan, loan.length, 0) }.isFailure) {
                assertNull(reader.frame())
                rejects { reader.read(0) }
            } else {
                val next = reader.read(0)
                if (next != null) assertTrue(next.offset >= 0 && next.length in 1..ConnectionWire.MAX_PAYLOAD)
            }
            reader.cancel()
        }
    }

    @Test fun t881IndependentWriterProgressesWhileNativeReaderIsBlocked() {
        val reader = ConnectionFrameReader(1000)
        val read = checkNotNull(reader.read(0))
        val writer = ConnectionFrameWriter(1000)
        val frame = data(4096)
        writer.begin(frame, 0)
        while (true) {
            val write = writer.write(1) ?: break
            writer.writeComplete(write, write.length, 1)
        }
        assertSame(frame, writer.flushComplete(checkNotNull(writer.flush(1)), 1))
        assertTrue(reader.nativePending())
        reader.tick(100000)
        assertNull(reader.read(100000))
        reader.cancel()
        rejects { reader.readComplete(read, 0, 100000) }
        assertFalse(reader.nativePending())
    }
}
