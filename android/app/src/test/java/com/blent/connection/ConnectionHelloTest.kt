package com.blent.connection

import org.junit.Assert.*
import org.junit.Test

class ConnectionHelloTest {
    @Test fun t846IdentityHeaderBoundsRolesAndMutations() {
        for (length in listOf(1, 255, 256, 4096)) for (host in listOf(false, true)) {
            val wire = ConnectionHello.encode(host, length)
            assertArrayEquals(byteArrayOf(66, 76, 80, 65, 1), wire.copyOf(5))
            assertEquals(if (host) 1 else 0, wire[5].toInt())
            assertEquals(length, ConnectionHello.decode(wire, host))
            assertThrows(IllegalArgumentException::class.java) { ConnectionHello.decode(wire, !host) }
        }
        for (size in listOf(Int.MIN_VALUE, 0, 4097, Int.MAX_VALUE)) {
            assertThrows(IllegalArgumentException::class.java) { ConnectionHello.encode(true, size) }
        }
        for (size in listOf(0, 1, 7, 9, 4096)) {
            assertThrows(IllegalArgumentException::class.java) { ConnectionHello.decode(ByteArray(size), true) }
        }
        val wire = ConnectionHello.encode(true, 4096)
        for (position in wire.indices) for (value in 0..255) {
            val changed = wire.copyOf().apply { this[position] = value.toByte() }
            runCatching { ConnectionHello.decode(changed, true) }.onSuccess {
                assertArrayEquals(changed, ConnectionHello.encode(true, it))
            }
        }
    }
}
