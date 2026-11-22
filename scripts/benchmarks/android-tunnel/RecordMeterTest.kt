package com.blent.connection

import java.io.ByteArrayOutputStream
import org.junit.Assert.*
import org.junit.Test

class RecordMeterTest {
    private fun record(kind: Int, size: Int) = byteArrayOf(kind.toByte(), 3, 3,
        (size ushr 8).toByte(), size.toByte()) + ByteArray(size) { 7 }

    @Test fun t1755RecordCountsSurviveEveryHeaderAndPayloadSplit() {
        val bytes = record(22, 32) + record(23, 0) + record(23, 4096) + record(23, 16384)
        for (chunk in listOf(1, 2, 3, 4, 5, 17, 4096, bytes.size)) {
            val output = ByteArrayOutputStream()
            val meter = RecordMeter(output)
            var offset = 0
            while (offset < bytes.size) {
                val length = minOf(chunk, bytes.size - offset)
                meter.write(bytes, offset, length)
                offset += length
            }
            meter.flush(); meter.close()
            assertEquals(3L, meter.records)
            assertArrayEquals(bytes, output.toByteArray())
        }
    }

    @Test fun t1755RecordBoundAndSingleByteWritesAreChecked() {
        val output = ByteArrayOutputStream()
        val meter = RecordMeter(output)
        val bytes = record(23, 18432)
        bytes.forEach { meter.write(it.toInt() and 255) }
        assertEquals(1L, meter.records)
        assertArrayEquals(bytes, output.toByteArray())
        assertThrows(IllegalStateException::class.java) { meter.write(record(23, 18433), 0, 5) }
    }

    @Test fun t1755InvalidExtentsAndOversizedRecordsAreRejected() {
        val random = java.util.Random(1755)
        repeat(128) {
            val meter = RecordMeter(ByteArrayOutputStream())
            val invalid = 18433 + random.nextInt(65536 - 18433)
            assertThrows(IllegalStateException::class.java) { meter.write(record(23, invalid), 0, 5) }
        }
        for (range in listOf(-1 to 1, 0 to -1, 0 to Int.MAX_VALUE, Int.MAX_VALUE to 1, 5 to 1)) {
            val meter = RecordMeter(ByteArrayOutputStream())
            assertThrows(IndexOutOfBoundsException::class.java) { meter.write(ByteArray(5), range.first, range.second) }
            assertEquals(0L, meter.records)
        }
    }

    private fun sockets(): Set<String> {
        val directory = java.nio.file.Paths.get("/proc/self/fd")
        java.nio.file.Files.newDirectoryStream(directory).use { entries ->
            return entries.mapNotNull { runCatching { java.nio.file.Files.readSymbolicLink(it).toString() }.getOrNull() }
                .filter { it.startsWith("socket:[") }.toSet()
        }
    }

    @Test fun t1755LayeredSocketClosesEveryOwnedDescriptor() {
        val before = sockets()
        java.net.ServerSocket(0, 1, java.net.InetAddress.getLoopbackAddress()).use { listener ->
            java.net.Socket(java.net.InetAddress.getLoopbackAddress(), listener.localPort).use { original ->
                listener.accept().use { peer ->
                    MeteredSocket(original).use { metered ->
                        metered.reuseAddress = true
                        metered.getOutputStream().write(1)
                        assertEquals(1, peer.getInputStream().read())
                    }
                    assertTrue(original.isClosed)
                }
            }
        }
        assertEquals(before, sockets())
    }

    @Test fun t1755RejectedOutputDoesNotAdvanceObservedRecords() {
        val failure = object : java.io.OutputStream() {
            override fun write(value: Int) { throw java.io.IOException("T1755 write rejected") }
        }
        val meter = RecordMeter(failure)
        assertThrows(java.io.IOException::class.java) { meter.write(record(23, 1)) }
        assertEquals(0L, meter.records)
    }
}
