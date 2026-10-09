package com.blent

import java.io.ByteArrayInputStream
import java.nio.ByteBuffer
import org.junit.Assert.*
import org.junit.Test

/** T822: plain JVM contracts, with no Android runner, codec or owner stubs. */
class PortableVideoContractTest {
    @Test fun t822_wireValuesAndBorrowedPacketDispatchAreIndependent() {
        val choice = DecoderSelection("codec", "h264", "baseline", 31, 8, true, 60)
        assertEquals("5:codec:h264:baseline:31:8:1:60", choice.receipt())
        assertEquals("5:codec:h264:baseline:31:8:0:0", choice.receipt(false))
        val bytes = ByteBuffer.allocate(14).putInt(10).put(1).putInt(-1).put(byteArrayOf(1,2,3,4,5)).array()
        val reader = VideoPacketReader(ByteArrayInputStream(bytes))
        assertTrue(reader.read())
        var delivered = false
        assertTrue(reader.dispatch(object : VideoPacketSink {
            override fun configuration(data: ByteArray, offset: Int, size: Int) { fail("unexpected configuration") }
            override fun frame(sequence: Int, data: ByteArray, offset: Int, size: Int) {
                assertEquals(-1, sequence)
                assertArrayEquals(byteArrayOf(1,2,3,4,5), data.copyOfRange(offset, offset + size))
                delivered = true
            }
        }))
        assertTrue(delivered)
        assertEquals("video/avc", DecoderFormat(VideoWire.MIME_TYPE, 640, 480, 30).mimeType)
    }

    @Test fun t822_profilesAndIdentifiersRejectBoundedInvalidCorpus() {
        for (codec in VideoCodec.types.keys) {
            assertFalse(VideoProfiles.profile(codec, "invalid", 8))
            assertFalse(VideoProfiles.profile(codec, "main", -1))
            for (level in -10..100) assertEquals(level in VideoProfiles.levels.getValue(codec), VideoProfiles.validLevel(codec, level))
        }
        assertTrue(VideoProfiles.profile("h264", "constrained-baseline", 8))
        assertTrue(VideoProfiles.profile("hevc", "main10", 10))
        assertTrue(VideoProfiles.profile("vp9", "profile2", 10))
        assertTrue(VideoProfiles.profile("av1", "main", 10))
        assertFalse(VideoProfiles.profile("unknown", "main", 8))
        assertFalse(VideoProfiles.validLevel("unknown", 31))
        for (code in 0..255) assertEquals(code in 33..126, VideoProfiles.identifier(code.toChar().toString()))
        for (length in 0..130) assertEquals(length in 1..128, VideoProfiles.identifier("a".repeat(length)))
    }

    @Test fun t822_endpointValidationRunsWithoutAndroid() {
        val audio = AudioEndpoint("a".repeat(64), 1234, 1, 1, 20, false)
        val camera = CameraEndpoint("a".repeat(64), 1234, 640, 480, 30, 1000)
        for (port in listOf(Int.MIN_VALUE, -1, 0, 1, 65535, 65536, Int.MAX_VALUE)) {
            assertEquals(port in 1..65535, audio.copy(port = port).valid())
            assertEquals(port in 1..65535, camera.copy(port = port).valid())
        }
        assertFalse(audio.copy(token = "x").valid())
        assertFalse(camera.copy(width = Int.MAX_VALUE).valid())
    }
}
