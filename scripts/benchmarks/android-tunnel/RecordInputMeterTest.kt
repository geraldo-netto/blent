package com.blent.connection

import java.io.ByteArrayInputStream
import java.io.ByteArrayOutputStream
import java.io.IOException
import java.io.InputStream
import org.junit.Assert.*
import org.junit.Test

class RecordInputMeterTest {
    private fun record(size: Int) = byteArrayOf(23, 3, 3, (size ushr 8).toByte(), size.toByte()) + ByteArray(size) { 7 }
    @Test fun t1770EveryReadFragmentPreservesBytesAndCountsOnce() {
        val bytes = record(0) + record(17) + record(16384)
        for (chunk in listOf(1, 2, 3, 4, 5, 17, 4096, 16384)) {
            val meter = RecordInputMeter(ByteArrayInputStream(bytes))
            val output = ByteArrayOutputStream()
            val buffer = ByteArray(chunk + 2)
            while (true) {
                val count = meter.read(buffer, 1, chunk)
                if (count < 0) break
                output.write(buffer, 1, count)
            }
            assertEquals(3L, meter.records)
            assertEquals(-1, meter.read()); assertEquals(3L, meter.records)
            assertArrayEquals(bytes, output.toByteArray())
            assertEquals(0, buffer.first().toInt()); assertEquals(0, buffer.last().toInt())
            meter.close()
        }
    }
    @Test fun t1770SingleByteSkipAndZeroReadsShareTheSameCounter() {
        val bytes = record(17) + record(64)
        val meter = RecordInputMeter(ByteArrayInputStream(bytes))
        repeat(5) { assertEquals(bytes[it].toInt() and 255, meter.read()) }
        assertEquals(1L, meter.records)
        assertEquals(0, meter.read(ByteArray(0), 0, 0))
        assertEquals((bytes.size - 5).toLong(), meter.skip((bytes.size - 5).toLong()))
        assertEquals(2L, meter.records); assertEquals(0, meter.available())
        assertFalse(meter.markSupported()); assertEquals(-1, meter.read())
    }
    @Test fun t1770InvalidExtentsAndRejectedInputNeverAdvanceAccounting() {
        val meter = RecordInputMeter(ByteArrayInputStream(record(1)))
        for ((offset, count) in listOf(-1 to 1, 0 to -1, 0 to Int.MAX_VALUE, Int.MAX_VALUE to 1, 5 to 1)) {
            assertThrows(IndexOutOfBoundsException::class.java) { meter.read(ByteArray(5), offset, count) }
            assertEquals(0L, meter.records)
        }
        assertEquals(23, meter.read())
        val failed = RecordInputMeter(object : InputStream() {
            override fun read(): Int = throw IOException("T1770 rejected native read")
        })
        assertThrows(IOException::class.java) { failed.read(ByteArray(5)) }
        assertEquals(0L, failed.records)
    }
    @Test fun t1770OversizedRecordsFailAndOwnedInputCloses() {
        var closed = false
        val input = object : ByteArrayInputStream(record(18433)) {
            override fun close() { closed = true; super.close() }
        }
        RecordInputMeter(input).use { meter ->
            assertThrows(IllegalStateException::class.java) { meter.read(ByteArray(5)) }
        }
        assertTrue(closed)
    }
}
