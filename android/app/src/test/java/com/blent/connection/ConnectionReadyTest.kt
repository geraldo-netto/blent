package com.blent.connection

import org.junit.Assert.*
import org.junit.Test

class ConnectionReadyTest {
    @Test fun t846ApprovalReadyHasExactRoleAndVersion() {
        assertEquals(8, ConnectionReady.BYTES)
        for (host in listOf(false, true)) {
            val marker = byteArrayOf(66, 76, 80, 82, 1, if (host) 1 else 0, 0, 0)
            assertArrayEquals(marker, ConnectionReady.encode(host))
            ConnectionReady.decode(marker, host)
            assertThrows(IllegalArgumentException::class.java) { ConnectionReady.decode(marker, !host) }
        }
    }
    @Test fun t846ReadyRejectsEveryMutatedByteAndInvalidLength() {
        val valid = ConnectionReady.encode(true)
        for (position in valid.indices) for (value in 0..255) {
            if (value == valid[position].toInt()) continue
            val changed = valid.copyOf().apply { this[position] = value.toByte() }
            assertThrows(IllegalArgumentException::class.java) { ConnectionReady.decode(changed, true) }
        }
        for (length in listOf(0, 1, 7, 9, 4096, 65536)) {
            assertThrows(IllegalArgumentException::class.java) { ConnectionReady.decode(valid.copyOf(length), true) }
        }
        val random = java.util.Random(846)
        repeat(256) {
            val invalid = ByteArray(random.nextInt(64)).apply { random.nextBytes(this) }
            assertThrows(IllegalArgumentException::class.java) { ConnectionReady.decode(invalid, true) }
        }
    }
}
