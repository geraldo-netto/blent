package com.blent.connection

import kotlinx.serialization.json.*
import org.junit.Assert.*
import org.junit.Test

class LocalMeasurementTest {
    @Test fun t1763BothCurrentConsumerAndDirectControlValidateExactBytesAndCloseOwners() {
        for (direct in listOf(false, true)) for (size in listOf(1, 4096, 65536)) {
            val result = localSample(size, 0, direct, 2)
            assertEquals(2, result.getValue("frames").jsonPrimitive.int)
            assertEquals(2, result.getValue("latency_ns").jsonArray.size)
            assertEquals("loopback_tcp_local_stream_analogue", result.getValue("transport").jsonPrimitive.content)
            assertEquals(0, result.getValue("tls_application_records").jsonPrimitive.int)
            assertTrue(result.getValue("driver_and_io_cpu_ns").jsonPrimitive.long > 0)
            assertEquals(4L, result.getValue("native_completions_including_ack").jsonPrimitive.long)
        }
    }
    @Test fun t1763FiniteTrialBoundsRejectBeforeNativeAdmission() {
        for (size in listOf(Int.MIN_VALUE, -1, 0, 65537, Int.MAX_VALUE)) {
            assertThrows(IllegalArgumentException::class.java) { localSample(size, 0, false, 1) }
        }
        for (frames in listOf(Int.MIN_VALUE, -1, 0, 61, Int.MAX_VALUE)) {
            assertThrows(IllegalArgumentException::class.java) { localSample(1, 0, false, frames) }
        }
        for (fps in listOf(Int.MIN_VALUE, -1, 1, 61, Int.MAX_VALUE)) {
            assertThrows(IllegalArgumentException::class.java) { localSample(1, fps, false, 1) }
        }
    }
    @Test fun t1763WrongByteControlFailsAndClosesTheOriginalSocket() {
        val connection = LocalMeasurement(1, 3)
        val failure = assertThrows(Exception::class.java) {
            connection.use {
                MeasuredIo(it).use { io ->
                    val loan = ConnectionWriteTransaction(ConnectionBuffer(byteArrayOf(7, 8, 7), 0, 3))
                    assertEquals(3, io.perform(ConnectionIoRequest.WriteFlushed(loan)))
                    io.acknowledgement()
                }
            }
        }
        assertTrue(failure.stackTraceToString().contains("T1763 local payload mismatch"))
        assertTrue(connection.socket.isClosed)
    }
}
