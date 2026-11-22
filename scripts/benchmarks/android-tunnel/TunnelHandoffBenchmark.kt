package com.blent.connection

import java.io.File
import java.util.concurrent.locks.LockSupport
import javax.net.ssl.SSLContext
import kotlinx.serialization.json.*

private const val FRAMES = 60

private fun framedWrite(io: MeasuredIo, writer: ConnectionFrameWriter, frame: ConnectionFrame) {
    writer.begin(frame, io.now())
    val loan = checkNotNull(writer.write(io.now()))
    val count = io.perform(ConnectionIoRequest.WriteFlushed(loan))
    check(writer.writeComplete(loan, count, io.now()) === frame)
}

/** One bounded copy per frame. Chunking preserves the current native request
 * limit; full 64-KiB payloads therefore still need two writes. */
private fun combinedWrite(io: MeasuredIo, frame: ConnectionFrame, staging: ByteArray, header: ByteArray) {
    ConnectionWire.encode(frame.header, header)
    header.copyInto(staging)
    frame.bytes.copyInto(staging, header.size, frame.offset, frame.offset + frame.length)
    val length = header.size + frame.length
    var offset = 0
    while (offset < length) {
        val count = minOf(ConnectionWire.MAX_PAYLOAD, length - offset)
        check(io.perform(ConnectionIoRequest.Write(ConnectionBuffer(staging, offset, count))) == count)
        offset += count
    }
}

private fun cpu(counters: NativeCounters, threads: List<Thread>): Long =
    (threads + Thread.currentThread()).sumOf { counters.read(it).cpu }

private fun pace(start: Long, frame: Int, fps: Int) {
    if (fps == 0) return
    val due = start + frame * (1_000_000_000L / fps)
    while (true) {
        val remaining = due - System.nanoTime()
        if (remaining <= 0) return
        LockSupport.parkNanos(remaining)
        check(!Thread.interrupted()) { "Measurement cancelled" }
    }
}

private fun sample(server: SSLContext, client: SSLContext, size: Int, fps: Int, combined: Boolean): JsonObject {
    TunnelMeasurement(server, client, FRAMES, size).use { connection ->
        MeasuredIo(connection, combined).use { io ->
            val counters = NativeCounters()
            val writer = ConnectionFrameWriter(5000)
            val header = ByteArray(ConnectionWire.HEADER_BYTES)
            val payload = ByteArray(size) { 7 }
            val staging = ByteArray(if (combined) ConnectionWire.HEADER_BYTES + ConnectionWire.MAX_PAYLOAD else 0)
            val latency = LongArray(FRAMES)
            val recordsBefore = connection.records
            val cpuBefore = cpu(counters, io.threads)
            val start = System.nanoTime()
            repeat(FRAMES) { sequence ->
                pace(start, sequence, fps)
                val frame = ConnectionFrame(ConnectionWire.Header(ConnectionWire.DATA, ConnectionWire.DISPLAY, 1, 1,
                    sequence.toLong(), size, 0), payload, 0, size)
                val begin = System.nanoTime()
                if (combined) combinedWrite(io, frame, staging, header) else framedWrite(io, writer, frame)
                io.acknowledgement()
                latency[sequence] = System.nanoTime() - begin
            }
            return result(size, fps, combined, System.nanoTime() - start, cpu(counters, io.threads) - cpuBefore,
                connection.records - recordsBefore, io, latency, connection.socket.session.protocol, connection.socket.session.cipherSuite)
        }
    }
}

private fun result(size: Int, fps: Int, combined: Boolean, elapsed: Long, cpu: Long,
                   records: Long, io: MeasuredIo, latency: LongArray, protocol: String, cipher: String): JsonObject = buildJsonObject {
    put("tls_protocol", protocol); put("tls_cipher", cipher)
    put("payload_bytes", size); put("fps", fps); put("combined", combined); put("frames", FRAMES)
    put("elapsed_ns", elapsed); put("driver_and_io_cpu_ns", cpu)
    put("tls_application_records", records); put("native_completions_including_ack", io.completions)
    put("driver_poll_calls", io.polls); put("driver_wait_calls", io.waits); put("ready_hints_observed", io.readyHints)
    put("latency_ns", JsonArray(latency.map { JsonPrimitive(it) }))
}

private fun persist(output: File, results: List<JsonObject>) {
    val pending = File(output.parentFile, output.name + ".pending")
    try {
        pending.writeText(JsonArray(results).toString())
        java.nio.file.Files.move(pending.toPath(), output.toPath(), java.nio.file.StandardCopyOption.ATOMIC_MOVE,
            java.nio.file.StandardCopyOption.REPLACE_EXISTING)
    } finally { pending.delete() }
}

internal fun measurementOutput(path: String): File {
    val output = File(path).absoluteFile
    requireNotNull(output.parentFile) { "Output must name a file below a directory" }.mkdirs()
    return output
}

fun main(arguments: Array<String>) {
    require(arguments.size == 2)
    val server = measurementTls(File(arguments[1]), "host", "tablet")
    val client = measurementTls(File(arguments[1]), "tablet", "host")
    val output = measurementOutput(arguments[0])
    val results = mutableListOf<JsonObject>()
    repeat(4) { trial ->
        for (fps in listOf(0, 60)) for (size in listOf(64, 4096, 16384, 65536)) {
            val order = if (trial % 2 == 0) listOf(false, true) else listOf(true, false)
            for (combined in order) {
                results.add(buildJsonObject {
                    put("warmup", trial == 0); put("trial", trial)
                    sample(server, client, size, fps, combined).forEach { (key, value) -> put(key, value) }
                })
                persist(output, results)
                println("Completed ${results.size}/64 trials; payload=$size fps=$fps combined=$combined")
            }
        }
    }
}
