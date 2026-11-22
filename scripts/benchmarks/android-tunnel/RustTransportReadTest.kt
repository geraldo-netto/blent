package com.blent.connection

import java.io.File
import javax.net.ssl.SSLContext
import kotlinx.serialization.json.*
import org.junit.Assert.*
import org.junit.Test

private fun sendCredit(io: MeasuredIo, channel: Int, sequence: Int, amount: Int) {
    val writer = ConnectionFrameWriter(5000, ConnectionWritePolicy())
    val frame = ConnectionFrame(ConnectionWire.Header(ConnectionWire.CREDIT, channel, 1, 1,
        sequence.toLong(), 0, amount), ByteArray(0), 0, 0)
    writer.begin(frame, io.now())
    val loan = checkNotNull(writer.write(io.now()))
    check(writer.writeComplete(loan, io.perform(ConnectionIoRequest.WriteFlushed(loan)), io.now()) === frame)
}

private fun rustReadSample(helper: File, fixtures: File, client: SSLContext,
                           size: Int, pipeline: Int, ahead: Boolean): JsonObject {
    val frames = 16
    RustReadConnection(helper, fixtures, client, size, frames, pipeline).use { connection ->
        MeasuredIo(connection).use { io ->
            val reader = MeasuredFrameReader(if (ahead) 16384 else 0)
            val before = connection.records
            sendCredit(io, ConnectionWire.MANAGEMENT, 0, size * pipeline)
            sendCredit(io, ConnectionWire.DISPLAY, 0, size * pipeline)
            repeat(frames) { index ->
                val frame = reader.next(io)
                validateReadFrame(frame, size, index + 1)
                reader.release(frame)
                sendCredit(io, ConnectionWire.DISPLAY, index + 1, size)
            }
            val result = buildJsonObject {
                put("payload_bytes", size); put("frames", frames); put("pipeline", pipeline)
                put("read_ahead_bytes", if (ahead) 16384 else 0); put("extra_copied_bytes", reader.copied)
                put("native_frame_reads", io.reads); put("incoming_tls_records", connection.records - before)
                put("peer", "production Rust broker Transport and Rustls; synthetic mux activation")
            }
            connection.finish(io)
            return result
        }
    }
}

class RustTransportReadTest {
    private fun helper() = File(checkNotNull(System.getProperty("blent.rustReadFixture")))
    private fun fixtures() = File("../../testdata/connection-tls")
    @Test fun t1770ActualRustTransportPreservesFramesAndBoundsForBothReadPolicies() {
        val output = measurementOutput(checkNotNull(System.getProperty("blent.rustReadOutput")))
        val client = measurementTls(fixtures(), "tablet", "host")
        val results = ArrayList<JsonObject>()
        for (pipeline in listOf(1, 4)) for (size in listOf(64, 4096, 16384, 65536)) {
            for (ahead in listOf(false, true)) {
                val result = rustReadSample(helper(), fixtures(), client, size, pipeline, ahead)
                val reads = result.getValue("native_frame_reads").jsonPrimitive.long
                assertTrue(reads in 16..96)
                assertEquals(if (ahead) 16L * (size + 40) else 0,
                    result.getValue("extra_copied_bytes").jsonPrimitive.long)
                assertTrue(result.getValue("incoming_tls_records").jsonPrimitive.long > 0)
                results.add(result); persistMeasurement(output, results)
                println("T1770 completed ${results.size}/16 Rust receive observations")
            }
        }
    }
    @Test fun t1770CallerLossClosesOriginalSocketAndRetiresOriginalHelper() {
        val client = measurementTls(fixtures(), "tablet", "host")
        val connection = RustReadConnection(helper(), fixtures(), client, 64, 16, 4)
        assertThrows(java.io.IOException::class.java) {
            connection.use { throw java.io.IOException("T1770 deliberate caller loss") }
        }
        assertFalse(connection.alive); assertTrue(connection.closed)
    }
    @Test fun t1770InvalidDomainAndRejectedHandshakeSettleConstructorOwners() {
        val valid = measurementTls(fixtures(), "tablet", "host")
        assertThrows(IllegalArgumentException::class.java) { RustReadConnection(helper(), fixtures(), valid, 0, 16, 4) }
        val invalid = measurementTls(fixtures(), "host", "tablet")
        assertThrows(javax.net.ssl.SSLException::class.java) { RustReadConnection(helper(), fixtures(), invalid, 64, 16, 4) }
    }
}
