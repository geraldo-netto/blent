package com.blent.connection

import java.io.ByteArrayInputStream
import java.io.ByteArrayOutputStream
import java.io.DataInputStream
import java.io.DataOutputStream
import java.io.IOException
import java.io.InputStream
import org.junit.Assert.*
import org.junit.Test

class AuthorityPeerAdmissionTest {
    private class PrefixOnly(private val length: Int) : InputStream() {
        private var position = 0
        var payloadReads = 0
        override fun read(): Int = when (position++) {
            0 -> length ushr 8
            1 -> length and 255
            else -> { payloadReads++; throw IOException("T1155 inadmissible payload read") }
        }
    }

    @Test fun t1155InvalidLengthRejectsBeforeReadingAnyPayload() {
        val counts = listOf(0, 1, 63, 65, 255, 256, 4096, 32768, 65535) +
            List(1024) { (it * 40503 + 17) and 65535 }
        for (count in counts.filter { it != 64 }) {
            val source = PrefixOnly(count)
            assertThrows("T1155 length=$count", IllegalArgumentException::class.java) {
                ConnectionAuthorityCodec.peer(DataInputStream(source))
            }
            assertEquals("T1155 length=$count read payload", 0, source.payloadReads)
        }
    }

    @Test fun t1155CanonicalBytesAndAllInvalidCharacterValues() {
        val peer = ConnectionPeer.parse("0123456789abcdef".repeat(4))
        val encoded = ByteArrayOutputStream().also { ConnectionAuthorityCodec.peer(DataOutputStream(it), peer) }.toByteArray()
        assertArrayEquals(byteArrayOf(0, 64) + peer.fingerprint.toByteArray(Charsets.US_ASCII), encoded)
        assertEquals(peer, ConnectionAuthorityCodec.peer(DataInputStream(ByteArrayInputStream(encoded))))
        for (length in 0 until encoded.size) {
            assertThrows(Exception::class.java) {
                ConnectionAuthorityCodec.peer(DataInputStream(ByteArrayInputStream(encoded.copyOf(length))))
            }
        }
        for (value in 0..255) checkCharacter(encoded, value)
    }

    private fun checkCharacter(encoded: ByteArray, value: Int) {
        val changed = encoded.copyOf().apply { this[2] = value.toByte() }
        val input = DataInputStream(ByteArrayInputStream(changed))
        if (value.toChar() in "0123456789abcdef") {
            assertEquals(value.toChar(), ConnectionAuthorityCodec.peer(input).fingerprint.first())
        } else {
            assertThrows(Exception::class.java) { ConnectionAuthorityCodec.peer(input) }
        }
    }
}
