package com.blent.connection

import java.io.File
import javax.net.ssl.SSLContext
import javax.net.ssl.SSLSocket
import kotlinx.serialization.json.*

private fun framedWrite(io: MeasuredIo, writer: ConnectionFrameWriter, frame: ConnectionFrame) {
    writer.begin(frame, io.now())
    val loan = checkNotNull(writer.write(io.now()))
    val count = io.perform(ConnectionIoRequest.WriteFlushed(loan))
    check(writer.writeComplete(loan, count, io.now()) === frame)
}

private fun receiveMeasurementFrames(connection: SSLSocket, frames: Int, payloadBytes: Int) {
    val input = connection.inputStream
    val headerBytes = ByteArray(ConnectionWire.HEADER_BYTES)
    repeat(frames) { sequence ->
        check(input.readNBytes(headerBytes, 0, headerBytes.size) == headerBytes.size)
        val header = ConnectionWire.decode(headerBytes)
        check(header == ConnectionWire.Header(ConnectionWire.DATA, ConnectionWire.DISPLAY, 1, 1,
            sequence.toLong(), payloadBytes, 0))
        val body = input.readNBytes(header.length)
        check(body.size == header.length && body.all { it == 7.toByte() })
        connection.outputStream.write(1)
    }
}

private fun sample(server: SSLContext, client: SSLContext, size: Int, fps: Int, combined: Boolean): JsonObject {
    TunnelMeasurement(server, client) { receiveMeasurementFrames(it, MEASUREMENT_FRAMES, size) }.use { connection ->
        MeasuredIo(connection).use { io ->
            val counters = NativeCounters()
            val policy = ConnectionWritePolicy(if (combined) 16384 else 0)
            val writer = ConnectionFrameWriter(5000, policy)
            val payload = ByteArray(size) { 7 }
            val latency = LongArray(MEASUREMENT_FRAMES)
            val recordsBefore = connection.records
            val cpuBefore = measurementCpu(counters, io.threads)
            val start = System.nanoTime()
            repeat(MEASUREMENT_FRAMES) { sequence ->
                paceMeasurement(start, sequence, fps)
                val frame = ConnectionFrame(ConnectionWire.Header(ConnectionWire.DATA, ConnectionWire.DISPLAY, 1, 1,
                    sequence.toLong(), size, 0), payload, 0, size)
                val begin = System.nanoTime()
                framedWrite(io, writer, frame)
                io.acknowledgement()
                latency[sequence] = System.nanoTime() - begin
            }
            return measurementResult(size, fps, combined, System.nanoTime() - start,
                measurementCpu(counters, io.threads) - cpuBefore, io, latency, buildJsonObject {
                    put("transport", "loopback_tls")
                    put("tls_protocol", connection.socket.session.protocol); put("tls_cipher", connection.socket.session.cipherSuite)
                    put("gather_payload_bytes", policy.gatherPayloadBytes)
                    put("tls_application_records", connection.records - recordsBefore)
                })
        }
    }
}

fun main(arguments: Array<String>) {
    require(arguments.size == 2)
    val server = measurementTls(File(arguments[1]), "host", "tablet")
    val client = measurementTls(File(arguments[1]), "tablet", "host")
    val output = measurementOutput(arguments[0])
    runMeasurements(output) { size, fps, combined -> sample(server, client, size, fps, combined) }
}
