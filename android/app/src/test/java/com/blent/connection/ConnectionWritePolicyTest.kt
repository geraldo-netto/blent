package com.blent.connection

import java.io.ByteArrayOutputStream
import org.junit.Assert.*
import org.junit.Test

class ConnectionWritePolicyTest {
    private fun frame(size: Int): ConnectionFrame = ConnectionFrame(
        ConnectionWire.Header(ConnectionWire.DATA, ConnectionWire.DISPLAY, 1, 1, 0, size, 0),
        ByteArray(size + 7) { (it * 17).toByte() }, 3, size,
    )
    private fun expected(frame: ConnectionFrame): ByteArray {
        val header = ByteArray(40); ConnectionWire.encode(frame.header, header)
        return header + frame.bytes.copyOfRange(frame.offset, frame.offset + frame.length)
    }
    @Test fun t1760PolicyBoundsRejectInvalidCapacityBeforeAllocation() {
        assertEquals(0, ConnectionWritePolicy().gatherPayloadBytes)
        for (limit in listOf(0, 1, 4096, 16384)) {
            val policy = ConnectionWritePolicy(limit)
            for (length in listOf(Int.MIN_VALUE, -1, 0, 1, limit, limit + 1, 65536, Int.MAX_VALUE)) {
                assertEquals(length > 0 && length <= limit, policy.gathers(length))
            }
        }
        val invalid = listOf(Int.MIN_VALUE, -1, 16385, Int.MAX_VALUE) + List(128) { 16385 + it }
        for (value in invalid) assertThrows(IllegalArgumentException::class.java) { ConnectionWritePolicy(value) }
    }
    @Test fun t1760GatherLimitIncludesPayloadAndFundsOnlyOneReusableHeaderPlusLimit() {
        val writer = ConnectionFrameWriter(1000, ConnectionWritePolicy(4096))
        var scratch: ByteArray? = null
        for (size in listOf(1, 4096, 2, 4095)) {
            val frame = frame(size); writer.begin(frame, 0)
            val loan = checkNotNull(writer.write(0))
            assertNull(loan.second); assertEquals(4136, loan.first.bytes.size)
            if (scratch != null) assertSame(scratch, loan.first.bytes)
            scratch = loan.first.bytes
            assertArrayEquals(expected(frame), scratch.copyOfRange(loan.first.offset, loan.first.offset + loan.first.length))
            assertSame(frame, writer.writeComplete(loan, loan.length, 0))
        }
    }
    @Test fun t1760LargeDisabledAndMetadataFramesRetainOriginalBorrowedStorage() {
        for ((limit, size) in listOf(0 to 1, 1 to 2, 4096 to 4097, 16384 to 65536)) {
            val writer = ConnectionFrameWriter(1000, ConnectionWritePolicy(limit))
            val frame = frame(size); writer.begin(frame, 0)
            val loan = checkNotNull(writer.write(0)); val payload = checkNotNull(loan.second)
            assertSame(frame.bytes, payload.bytes); assertEquals(frame.offset, payload.offset)
            assertEquals(40, loan.first.bytes.size); assertSame(frame, writer.writeComplete(loan, loan.length, 0))
        }
        val writer = ConnectionFrameWriter(1000, ConnectionWritePolicy(16384))
        val metadata = ConnectionFrame(ConnectionWire.Header(ConnectionWire.CREDIT, 1, 1, 1, 0, 0, 1), byteArrayOf(), 0, 0)
        writer.begin(metadata, 0)
        val loan = checkNotNull(writer.write(0)); assertNull(loan.second)
        assertEquals(40, loan.first.bytes.size); assertSame(metadata, writer.writeComplete(loan, 40, 0))
    }
    private fun fragmented(size: Int, chunk: Int) {
        val writer = ConnectionFrameWriter(1000, ConnectionWritePolicy(16384))
        val frame = frame(size); writer.begin(frame, 0)
        val output = ByteArrayOutputStream()
        while (true) {
            val loan = writer.write(1) ?: break
            assertNull(writer.write(1)); assertNull(loan.second)
            val count = minOf(chunk, loan.length)
            output.write(loan.first.bytes, loan.first.offset, count)
            val finished = writer.writeComplete(loan, count, 1)
            if (count == loan.length) assertSame(frame, finished) else assertNull(finished)
            assertThrows(IllegalArgumentException::class.java) { writer.writeComplete(loan, count, 1) }
        }
        assertArrayEquals(expected(frame), output.toByteArray()); assertFalse(writer.nativePending())
    }
    @Test fun t1760BoundedPartialPrefixesPreserveEveryByteAndSingleFlushCompletion() {
        val random = java.util.Random(1760)
        repeat(128) { fragmented(1 + random.nextInt(16384), 1 + random.nextInt(257)) }
        for (chunk in listOf(1, 39, 40, 41, 16384, 16424)) fragmented(16384, chunk)
    }
    @Test fun t1760CancellationAndDeadlineNeverPermitScratchReuse() {
        for (cancel in listOf(false, true)) {
            val writer = ConnectionFrameWriter(10, ConnectionWritePolicy(16384))
            val frame = frame(4096); writer.begin(frame, 0)
            val loan = checkNotNull(writer.write(0)); val original = loan.first.bytes.copyOf()
            if (cancel) writer.cancel() else assertThrows(IllegalStateException::class.java) { writer.tick(10) }
            assertThrows(IllegalStateException::class.java) { writer.begin(frame(1), 10) }
            assertArrayEquals(original, loan.first.bytes); assertTrue(writer.nativePending())
            assertThrows(IllegalStateException::class.java) { writer.writeComplete(loan, loan.length, 10) }
            assertFalse(writer.nativePending())
            assertThrows(IllegalStateException::class.java) { writer.write(10) }
        }
    }
    @Test fun t1760SettingsAndWorkerSnapshotPreserveChoiceAndRejectObsoleteSchemas() {
        for (limit in listOf(0, 1, 4096, 16384)) {
            val spec = ConnectionWorkerSpec("usb", io = ConnectionRunLimits(write = ConnectionWritePolicy(limit)))
            val saved = ConnectionPreferences.encode(spec)
            assertTrue(saved.startsWith("2\n"))
            assertEquals(spec, ConnectionPreferences.decode(saved))
            assertEquals(limit.toString(), ConnectionPreferences.fields(spec).getValue("write_gather_bytes"))
            assertThrows(IllegalArgumentException::class.java) { ConnectionPreferences.decode(saved.replaceFirst("2\n", "1\n")) }
            val encoded = ConnectionWorkerCodec.spec(spec)
            assertEquals(3, encoded[0].toInt()); assertEquals(spec, ConnectionWorkerCodec.spec(encoded))
            for (obsolete in listOf(1, 2)) {
                assertThrows(IllegalArgumentException::class.java) { ConnectionWorkerCodec.spec(encoded.copyOf().also { it[0] = obsolete.toByte() }) }
            }
        }
    }
}
