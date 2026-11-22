package com.blent.connection

import kotlinx.serialization.json.*

private fun consumeLocal(io: MeasuredIo, consumer: LocalChannelConsumer, endpoint: LocalMeasurementEndpoint,
                         payload: ByteArray, size: Int) {
    endpoint.prepare(payload, 3, size)
    while (!endpoint.finished()) {
        consumer.advance(endpoint, endpoint.lease, ConnectionWire.CONTROL, io.port, io.now())
        consumer.complete(io.completion(), endpoint, endpoint.lease, ConnectionWire.CONTROL, false, io.now())
    }
}

internal fun localSample(size: Int, fps: Int, direct: Boolean, frames: Int = MEASUREMENT_FRAMES): JsonObject {
    require(fps == 0 || fps == 60)
    LocalMeasurement(frames, size).use { connection ->
        MeasuredIo(connection).use { io ->
            val counters = NativeCounters()
            val consumer = LocalChannelConsumer(ConnectionWire.MAX_PAYLOAD, 5000)
            val endpoint = LocalMeasurementEndpoint()
            val payload = ByteArray(size + 6) { 91 }.also { it.fill(7, 3, 3 + size) }
            val latency = LongArray(frames)
            val before = measurementSnapshot(counters, io.threads)
            val start = System.nanoTime()
            repeat(frames) { sequence ->
                paceMeasurement(start, sequence, fps)
                val begin = System.nanoTime()
                if (direct) {
                    val transaction = ConnectionWriteTransaction(ConnectionBuffer(payload, 3, size))
                    check(io.perform(ConnectionIoRequest.WriteFlushed(transaction)) == size)
                } else consumeLocal(io, consumer, endpoint, payload, size)
                io.acknowledgement()
                latency[sequence] = System.nanoTime() - begin
            }
            return measurementResult(size, fps, direct, System.nanoTime() - start,
                measurementDelta(counters, io.threads, before), io, latency, buildJsonObject {
                    put("transport", "loopback_tcp_local_stream_analogue")
                    put("mode", if (direct) "direct_native_transaction" else "production_local_consumer")
                    // The transport is explicitly plaintext; no TLS records exist.
                    put("tls_application_records", 0)
                })
        }
    }
}

fun main(arguments: Array<String>) {
    require(arguments.size == 1)
    runMeasurements(measurementOutput(arguments[0])) { size, fps, direct -> localSample(size, fps, direct) }
}
