package com.blent

import java.io.ByteArrayInputStream
import java.io.EOFException
import java.nio.ByteBuffer
import org.junit.Assert.*
import org.junit.Test

/** T1040: header rejection must precede payload I/O and backing growth. */
class VideoPacketValidationTest {
    private class Input(size: Int, type: Int) : ByteArrayInputStream(
        ByteBuffer.allocate(21).putInt(size).put(type.toByte()).array()
    ) {
        var largestBuffer = 0
        var largestRead = 0
        val consumed: Int get() = pos
        override fun read(buffer: ByteArray, offset: Int, length: Int): Int {
            largestBuffer = maxOf(largestBuffer, buffer.size)
            largestRead = maxOf(largestRead, length)
            return super.read(buffer, offset, length)
        }
    }

    private fun backing(reader: VideoPacketReader): ByteArray {
        val field = VideoPacketReader::class.java.getDeclaredField("data")
        field.isAccessible = true
        return field.get(reader) as ByteArray
    }

    private fun rejectedHeader(size: Int, type: Int, enabled: Boolean, consumed: Int) {
        val input = Input(size, type)
        val diagnostics = mutableListOf<String>()
        val reader = VideoPacketReader(input, enabled, diagnostics::add)
        val original = backing(reader)
        assertFalse("T1040 size=$size type=$type enabled=$enabled", reader.read())
        assertEquals("T1040 payload consumed after invalid header", consumed, input.consumed)
        assertSame("T1040 invalid header replaced backing", original, backing(reader))
        assertTrue("T1040 invalid header requested oversized storage", input.largestBuffer <= original.size)
        assertTrue("T1040 invalid header requested payload", input.largestRead <= 4)
        assertEquals(1, diagnostics.size)
    }

    @Test fun t1040_unknownTypesRejectBeforePayloadReadOrAllocation() {
        for (type in 3..255) {
            rejectedHeader(VideoWire.MAX_FRAME_SIZE + 1, type, true, 5)
        }
    }

    @Test fun t1040_typeSpecificLengthsRejectFromHeaderAlone() {
        for (size in listOf(3, 5, 512 * 1024 + 1, VideoWire.MAX_FRAME_SIZE + 1)) {
            rejectedHeader(size, 2, true, 5)
        }
        for (size in 2..5) rejectedHeader(size, 1, true, 5)
        rejectedHeader(2, 2, false, 5)
    }

    @Test fun t1040_genericBoundsRejectWithoutReadingType() {
        for (size in listOf(Int.MIN_VALUE, -1, 0, 1, VideoWire.MAX_FRAME_SIZE + 2, Int.MAX_VALUE)) {
            rejectedHeader(size, 0, true, 4)
        }
    }

    @Test fun t1040_validHeadersStillRequireCompletePayload() {
        for ((size, type) in listOf(2 to 0, 6 to 1, 2 to 2, (VideoWire.MAX_FRAME_SIZE + 1) to 0)) {
            val prefix = ByteBuffer.allocate(5).putInt(size).put(type.toByte()).array()
            val reader = VideoPacketReader(prefix.inputStream(), true)
            assertThrows("T1040 valid partial payload must remain EOF", EOFException::class.java) { reader.read() }
        }
    }
}
