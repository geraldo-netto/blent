package com.blent

import java.io.ByteArrayInputStream
import java.io.ByteArrayOutputStream
import java.io.InputStream
import java.nio.ByteBuffer
import java.util.Random
import org.junit.Assert.*
import org.junit.Test

class AoaProbeWireTest {
    private fun frame(sequence: Int = 0, session: Long = 1, payload: ByteArray = byteArrayOf(1)): ByteArray {
        val bytes = ByteBuffer.allocate(24 + payload.size)
        bytes.putInt(AoaProbeWire.MAGIC).putLong(session).putInt(sequence).putInt(payload.size)
            .putInt(AoaProbeWire.checksum(payload, payload.size)).put(payload)
        return bytes.array()
    }

    @Test fun t846_wireMatchesIndependentBigEndianCVector() {
        val raw = "42414f310123456789abcdef000000000000000355bc801d"
            .chunked(2).map { it.toInt(16).toByte() }.toByteArray()
        assertEquals(AoaProbeWire.Header(0x0123456789abcdefL, 0, 3, 0x55bc801d), AoaProbeWire.decode(raw))
        assertEquals(0x55bc801d, AoaProbeWire.checksum(byteArrayOf(1, 2, 3), 3))
    }

    @Test fun t846_rejectsMalformedAndOutOfBoundsHeaderFields() {
        val valid = frame().copyOf(24)
        for (size in listOf(0, 1, 23, 25, 65536)) {
            assertThrows(IllegalArgumentException::class.java) { AoaProbeWire.decode(ByteArray(size)) }
        }
        val invalid = listOf(0 to 0, 12 to -1, 12 to 4096, 16 to -1, 16 to 0, 16 to 65537, 16 to Int.MAX_VALUE)
        for ((offset, value) in invalid) {
            val raw = valid.copyOf().also { ByteBuffer.wrap(it).putInt(offset, value) }
            assertThrows(IllegalArgumentException::class.java) { AoaProbeWire.decode(raw) }
        }
        assertThrows(IllegalArgumentException::class.java) { AoaProbeWire.decode(frame(session = 0).copyOf(24)) }
    }

    @Test fun t846_boundedHeaderPropertiesExerciseSignedBoundaries() {
        val random = Random(846)
        repeat(512) {
            val length = random.nextInt()
            val raw = frame(session = Long.MIN_VALUE).copyOf(24)
            ByteBuffer.wrap(raw).putInt(12, 4095).putInt(16, length)
            if (length in 1..65536) assertEquals(length, AoaProbeWire.decode(raw).length)
            else assertThrows(IllegalArgumentException::class.java) { AoaProbeWire.decode(raw) }
        }
        for (length in listOf(1, 65536)) {
            val raw = frame().copyOf(24).also { ByteBuffer.wrap(it).putInt(16, length) }
            assertEquals(length, AoaProbeWire.decode(raw).length)
        }
    }

    @Test fun t846_partialReadsStopAtFinitePacketBudgetWithExactEcho() {
        val bytes = ByteArrayOutputStream()
        repeat(AoaProbeWire.MAX_PACKETS) { bytes.write(frame(it)) }
        val input = object : ByteArrayInputStream(bytes.toByteArray()) {
            override fun read(target: ByteArray, offset: Int, length: Int): Int =
                super.read(target, offset, minOf(length, 3))
        }
        val output = ByteArrayOutputStream()
        val reports = mutableListOf<Int>()
        assertEquals(4096L, AoaProbeWire.echo(input, output) { count, _ -> reports.add(count) })
        assertArrayEquals(bytes.toByteArray(), output.toByteArray())
        assertEquals((1..4096 step 64).toList(), reports)
    }

    @Test fun t846_staleSequenceSessionChecksumAndTruncationNeverEchoInvalidFrame() {
        val valid = frame()
        val corrupt = frame().apply { this[lastIndex] = 2 }
        for (bad in listOf(frame(sequence = 1), corrupt, valid.copyOf(23), valid.copyOf(24))) {
            val output = ByteArrayOutputStream()
            assertThrows(IllegalStateException::class.java) { AoaProbeWire.echo(ByteArrayInputStream(bad), output) { _, _ -> } }
            assertEquals(0, output.size())
        }
        for (second in listOf(frame(), frame(sequence = 1, session = 2))) {
            val output = ByteArrayOutputStream()
            assertThrows(IllegalStateException::class.java) { AoaProbeWire.echo(ByteArrayInputStream(valid + second), output) { _, _ -> } }
            assertArrayEquals(valid, output.toByteArray())
        }
    }

    @Test fun t846_zeroProgressInvalidCountsAndInvalidBufferLengthsAreBounded() {
        for (count in listOf(-1, 0, 5, Int.MAX_VALUE)) {
            val input = object : InputStream() {
                override fun read() = error("Bulk read required")
                override fun read(bytes: ByteArray, offset: Int, length: Int) = count
            }
            assertThrows(IllegalStateException::class.java) { AoaProbeWire.readFully(input, ByteArray(4), 4) }
        }
        for (length in listOf(-1, 5, Int.MAX_VALUE)) {
            assertThrows(IllegalArgumentException::class.java) { AoaProbeWire.readFully(ByteArrayInputStream(byteArrayOf()), ByteArray(4), length) }
            assertThrows(IllegalArgumentException::class.java) { AoaProbeWire.checksum(ByteArray(4), length) }
        }
    }
}
