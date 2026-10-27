package com.blent

import java.nio.ByteBuffer
import okio.Buffer
import org.junit.Assert.*
import org.junit.Test

class AudioProtocolVersionTest {
    private fun endpoint() = AudioEndpoint("a".repeat(64), 12345, 1, 1, 40, false)
    private fun grant(magic: String) = ByteBuffer.allocate(92)
        .put(magic.toByteArray()).put("b".repeat(64).toByteArray()).putLong(1)
        .put(1).put(1).putInt(48000).putShort(480).put(1).put(0).putShort(40).array()

    @Test fun t1298DefaultRequestUsesCurrentProtocol() {
        val request = Buffer()
        AudioWire(endpoint()).request(request, 7, true)
        assertEquals("BLAUREQ2", request.readUtf8(8))
    }

    @Test fun t1298OldGrantRejectsBeforePcm() {
        assertThrows(IllegalArgumentException::class.java) {
            AudioWire(endpoint()).negotiate(Buffer().writeUtf8(endpoint().token).write(grant("BLAUD001")))
        }
    }

    @Test fun t1298MissingNativeClockStillHasCurrentFixedPrefix() {
        val wire = AudioWire(endpoint())
        wire.negotiate(Buffer().writeUtf8(endpoint().token).write(grant("BLAUD002")))
        val output = Buffer()
        wire.send(output, ShortArray(480) { 17 }, 1, 100)
        val bytes = output.readByteArray()
        assertEquals(1012, bytes.size)
        assertArrayEquals(ByteArray(24), bytes.copyOfRange(28, 52))
        val legacy = bytes.copyOfRange(0, 28) + bytes.copyOfRange(52, bytes.size)
        ByteBuffer.wrap(legacy).putShort(24, 960)
        assertThrows(IllegalArgumentException::class.java) { wire.receive(Buffer().write(legacy)) }
        val block = wire.receive(Buffer().write(bytes))
        assertNull(block.clock)
        assertTrue(block.samples.all { it == 17.toShort() })
    }
}
