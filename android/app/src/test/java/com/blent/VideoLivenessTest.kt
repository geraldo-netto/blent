package com.blent

import java.io.ByteArrayInputStream
import java.io.EOFException
import java.nio.ByteBuffer
import org.junit.Assert.*
import org.junit.Test

/** T1038: transport messages never grant frame/decoder/render authority. */
class VideoLivenessTest {
    private class Sink : VideoPacketSink {
        var configurations = 0
        var frames = 0
        override fun configuration(data: ByteArray, offset: Int, size: Int) { configurations++ }
        override fun frame(sequence: Int, data: ByteArray, offset: Int, size: Int) { frames++ }
    }
    private fun packet(payload: ByteArray): ByteArray =
        ByteBuffer.allocate(5 + payload.size).putInt(1 + payload.size).put(2).put(payload).array()

    @Test fun t1038_goldenLivenessDoesNotDispatchOrReplaceFollowingMedia() {
        val golden = byteArrayOf(0, 0, 0, 2, 2, 1)
        val config = byteArrayOf(0, 0, 0, 3, 0, 7, 8)
        val frame = byteArrayOf(0, 0, 0, 6, 1, 0, 0, 0, 9, 42)
        val reader = VideoPacketReader(ByteArrayInputStream(golden + golden + config + frame), livenessEnabled = true)
        val sink = Sink()
        repeat(2) { assertTrue(reader.read()); assertTrue("T1038 valid liveness rejected", reader.dispatch(sink)) }
        assertEquals(0, sink.configurations)
        assertEquals(0, sink.frames)
        assertTrue(reader.read()); assertTrue(reader.dispatch(sink))
        assertEquals(1, sink.configurations)
        assertEquals(0, sink.frames)
        assertTrue(reader.read()); assertTrue(reader.dispatch(sink))
        assertEquals(1, sink.frames)
    }
    @Test fun t1038_boundedLengthsAndVersionsRejectMalformedLivenessWithoutMediaCallbacks() {
        for (length in 0..32) {
            for (version in 0..255) {
                val reader = VideoPacketReader(packet(ByteArray(length) { version.toByte() }).inputStream(), livenessEnabled = true)
                val sink = Sink()
                val accepted = reader.read() && reader.dispatch(sink)
                assertEquals("T1038 length=$length version=$version", length == 1 && version == 1, accepted)
                assertEquals(0, sink.configurations)
                assertEquals(0, sink.frames)
            }
        }
    }
    @Test fun t1038_fragmentedAndTruncatedLivenessCannotBecomeAPartialFrame() {
        val golden = byteArrayOf(0, 0, 0, 2, 2, 1)
        for (length in golden.indices) {
            val reader = VideoPacketReader(ByteArrayInputStream(golden.copyOf(length)), livenessEnabled = true)
            val sink = Sink()
            assertThrows(EOFException::class.java) { if (reader.read()) reader.dispatch(sink) }
            assertEquals(0, sink.frames)
        }
        val input = object : ByteArrayInputStream(golden) {
            override fun read(bytes: ByteArray, offset: Int, size: Int): Int = super.read(bytes, offset, minOf(size, 1))
        }
        val reader = VideoPacketReader(input, livenessEnabled = true)
        assertTrue(reader.read())
        assertTrue(reader.dispatch(Sink()))
    }
    @Test fun t1039_absentNegotiationRejectsNewPacketTypeWithoutMediaCallbacks() {
        val reader = VideoPacketReader(byteArrayOf(0, 0, 0, 2, 2, 1).inputStream())
        val sink = Sink()
        assertTrue(reader.read())
        assertFalse("T1039 unnegotiated liveness accepted", reader.dispatch(sink))
        assertEquals(0, sink.frames)
        assertEquals(0, sink.configurations)
    }

}
