package com.blent.connection

import java.io.File
import kotlinx.serialization.json.*
import org.junit.Assert.*
import org.junit.Test

class ReadMeasurementTest {
    @Test fun t1757StagingRetainsExactLoanAndRejectsReuseBeforeSettlement() {
        val staging = ReadStaging(16)
        val loan = staging.read()
        loan.bytes.fill(7)
        assertThrows(IllegalStateException::class.java) { staging.read() }
        assertThrows(IllegalStateException::class.java) { staging.copyTo(ConnectionBuffer(ByteArray(4), 0, 4)) }
        assertThrows(IllegalArgumentException::class.java) { staging.complete(ConnectionBuffer(loan.bytes, 0, 16), 16) }
        for (count in listOf(Int.MIN_VALUE, -1, 0, 17, Int.MAX_VALUE)) {
            assertThrows(IllegalArgumentException::class.java) { staging.complete(loan, count) }
            assertThrows(IllegalStateException::class.java) { staging.read() }
        }
        staging.complete(loan, 16)
        assertThrows(IllegalArgumentException::class.java) { staging.complete(loan, 16) }
        assertThrows(IllegalStateException::class.java) { staging.read() }
        val target = ByteArray(20) { 91 }
        assertEquals(16, staging.copyTo(ConnectionBuffer(target, 2, 16)))
        assertArrayEquals(byteArrayOf(91, 91) + ByteArray(16) { 7 } + byteArrayOf(91, 91), target)
        assertEquals(16L, staging.copied); assertTrue(staging.empty())
        assertSame(loan.bytes, staging.read().bytes)
    }
    @Test fun t1757FragmentedCopiesPreserveEveryByteAndBound() {
        val random = java.util.Random(1757)
        repeat(128) {
            val capacity = 1 + random.nextInt(16384)
            val staging = ReadStaging(capacity)
            val loan = staging.read()
            random.nextBytes(loan.bytes)
            val length = 1 + random.nextInt(capacity)
            val expected = loan.bytes.copyOf(length)
            staging.complete(loan, length)
            val actual = java.io.ByteArrayOutputStream()
            while (!staging.empty()) {
                val output = ByteArray(2 + random.nextInt(128))
                val count = staging.copyTo(ConnectionBuffer(output, 1, output.size - 1))
                actual.write(output, 1, count)
            }
            assertArrayEquals(expected, actual.toByteArray())
            assertEquals(length.toLong(), staging.copied)
        }
    }
    @Test fun t1757StagingRejectsInvalidCapacityAndDestinationExtents() {
        for (size in listOf(Int.MIN_VALUE, -1, 0, 65537, Int.MAX_VALUE)) {
            assertThrows(IllegalArgumentException::class.java) { ReadStaging(size) }
        }
        val staging = ReadStaging(8)
        val loan = staging.read(); staging.complete(loan, 8)
        for ((offset, size) in listOf(-1 to 1, 0 to 0, 0 to -1, 0 to 5, 4 to 1, Int.MAX_VALUE to 1)) {
            assertThrows(IllegalArgumentException::class.java) { staging.copyTo(ConnectionBuffer(ByteArray(4), offset, size)) }
        }
        assertEquals(0L, staging.copied)
    }
    @Test fun t1757PeerHeaderAndPayloadFaultsCannotPassValidation() {
        val header = ConnectionWire.Header(ConnectionWire.DATA, ConnectionWire.DISPLAY, 1, 1, 0, 3, 0)
        assertThrows(IllegalStateException::class.java) { validateReadFrame(ConnectionFrame(header, byteArrayOf(7, 8, 7), 0, 3), 3, 0) }
        assertThrows(IllegalStateException::class.java) { validateReadFrame(ConnectionFrame(header, ByteArray(3) { 7 }, 0, 3), 3, 1) }
    }
    @Test fun t1757CurrentFramingAndReadAheadUseActualTlsAndRetireNativeWorkers() {
        val fixtures = File("../../testdata/connection-tls")
        val server = measurementTls(fixtures, "host", "tablet")
        val client = measurementTls(fixtures, "tablet", "host")
        for (gather in listOf(0, 16384)) for (ahead in listOf(false, true)) for (size in listOf(64, 4096, 16384, 65536)) {
            val result = readSample(server, client, size, 0, ahead, gather, 2)
            assertEquals(2, result.getValue("frames").jsonPrimitive.int)
            assertEquals("TLSv1.3", result.getValue("tls_protocol").jsonPrimitive.content)
            assertEquals(if (ahead) 2L * (size + 40) else 0L, result.getValue("extra_copied_bytes").jsonPrimitive.long)
            assertEquals(2L, result.getValue("tls_application_records").jsonPrimitive.long)
            assertTrue(result.getValue("driver_and_io_allocated_bytes").jsonPrimitive.long > 0)
            assertTrue(result.getValue("native_completions_including_ack").jsonPrimitive.long >= 4)
        }
    }
}
