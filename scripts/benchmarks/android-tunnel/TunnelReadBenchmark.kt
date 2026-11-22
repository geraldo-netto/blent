package com.blent.connection

import java.io.File
import javax.net.ssl.SSLContext
import javax.net.ssl.SSLSocket
import kotlinx.serialization.json.*

/** Peer preparation is outside measured driver CPU. Production framing/policy
 * generate the actual borrowed/gathered slices; the peer is a synchronous sender. */
private fun sendReadFrames(socket: SSLSocket, frames: Int, size: Int, policy: ConnectionWritePolicy) {
    val writer = ConnectionFrameWriter(5000, policy)
    val payload = ByteArray(size) { 7 }
    repeat(frames) { sequence ->
        // A request token paces the peer and includes both directions in latency.
        check(socket.inputStream.read() == 1)
        val frame = ConnectionFrame(ConnectionWire.Header(ConnectionWire.DATA, ConnectionWire.DISPLAY,
            1, 1, sequence.toLong(), size, 0), payload, 0, size)
        writer.begin(frame, 0)
        val loan = checkNotNull(writer.write(0))
        socket.outputStream.write(loan.first.bytes, loan.first.offset, loan.first.length)
        loan.second?.let { socket.outputStream.write(it.bytes, it.offset, it.length) }
        socket.outputStream.flush()
        check(writer.writeComplete(loan, loan.length, 0) === frame)
    }
}

internal fun validateReadFrame(frame: ConnectionFrame, size: Int, sequence: Int) {
    check(frame.header == ConnectionWire.Header(ConnectionWire.DATA, ConnectionWire.DISPLAY,
        1, 1, sequence.toLong(), size, 0)) { "T1757 wrong frame header" }
    check(frame.length == size)
    repeat(size) { check(frame.bytes[frame.offset + it] == 7.toByte()) { "T1757 wrong payload byte" } }
}

private fun requestReadFrame(io: MeasuredIo) {
    val request = ConnectionIoRequest.WriteFlushed(ConnectionWriteTransaction(ConnectionBuffer(byteArrayOf(1), 0, 1)))
    check(io.perform(request) == 1)
}

internal fun readSample(server: SSLContext, client: SSLContext, size: Int, fps: Int,
                        ahead: Boolean, gather: Int, frames: Int = MEASUREMENT_FRAMES): JsonObject {
    require(size in 1..ConnectionWire.MAX_PAYLOAD)
    require(frames in 1..MEASUREMENT_FRAMES)
    require(fps == 0 || fps == 60)
    val policy = ConnectionWritePolicy(gather)
    TunnelMeasurement(server, client) { sendReadFrames(it, frames, size, policy) }.use { connection ->
        MeasuredIo(connection).use { io ->
            val counters = NativeCounters()
            val reader = MeasuredFrameReader(if (ahead) 16384 else 0)
            val latency = LongArray(frames)
            val recordsBefore = connection.records
            val cpuBefore = measurementCpu(counters, io.threads)
            val start = System.nanoTime()
            repeat(frames) { sequence ->
                paceMeasurement(start, sequence, fps)
                val begin = System.nanoTime()
                requestReadFrame(io)
                val frame = reader.next(io)
                validateReadFrame(frame, size, sequence)
                reader.release(frame)
                latency[sequence] = System.nanoTime() - begin
            }
            return measurementResult(size, fps, ahead, System.nanoTime() - start,
                measurementCpu(counters, io.threads) - cpuBefore, io, latency, buildJsonObject {
                    put("transport", "loopback_tls_read")
                    put("tls_protocol", connection.socket.session.protocol); put("tls_cipher", connection.socket.session.cipherSuite)
                    put("peer_gather_payload_bytes", gather); put("read_ahead_bytes", if (ahead) 16384 else 0)
                    put("extra_copied_bytes", reader.copied)
                    put("tls_application_records", connection.records - recordsBefore)
                    put("record_direction", "outgoing_request_tokens_only")
                })
        }
    }
}

fun main(arguments: Array<String>) {
    require(arguments.size == 3)
    val server = measurementTls(File(arguments[1]), "host", "tablet")
    val client = measurementTls(File(arguments[1]), "tablet", "host")
    val gather = arguments[2].toInt()
    ConnectionWritePolicy(gather)
    runMeasurements(measurementOutput(arguments[0])) { size, fps, ahead -> readSample(server, client, size, fps, ahead, gather) }
}
