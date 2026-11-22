package com.blent.connection

import kotlinx.serialization.json.*

internal const val PRODUCER_ALLOCATION_ITERATIONS = 1024

/** Synchronous synthetic native owner. One original read is retained until the
 * named completion, including cancellation; no real native operation is claimed. */
private class ProducerMeasurementIo : ConnectionIo {
    var pending: ConnectionIoRequest.Read? = null; private set
    private var stopped = false
    override fun submit(request: ConnectionIoRequest): Boolean {
        check(!stopped && pending == null)
        pending = request as ConnectionIoRequest.Read
        return true
    }
    fun finish(count: Int): ConnectionIoCompletion {
        val request = checkNotNull(pending)
        require(count in 0..request.loan.length)
        request.loan.bytes.fill(7, request.loan.offset, request.loan.offset + count)
        pending = null
        return ConnectionIoCompletion(request, count)
    }
    override fun poll(): ConnectionIoCompletion? = error("T1766 fixture completes only original reads explicitly")
    override fun stop() { stopped = true }
    override fun retired(): Boolean = stopped && pending == null
    override fun closeFailure(): Exception? = null
}

private class ProducerMeasurementEndpoint(private val base: LocalMeasurementEndpoint = LocalMeasurementEndpoint()) :
    ConnectionEndpoints by base {
    val lease = base.lease
    var owned: ByteArray? = null; private set
    override fun offer(lease: ConnectionResourceLease, channel: Int, bytes: ByteArray): Boolean {
        check(lease === this.lease && channel == ConnectionWire.CONTROL)
        check(owned == null)
        owned = bytes
        return true
    }
    fun consume(size: Int) {
        val bytes = checkNotNull(owned)
        check(bytes.size == size && bytes.all { it == 7.toByte() })
        owned = null
    }
    fun discard() { owned = null }
}

internal class ProducerAllocationTrial(private val size: Int) : AutoCloseable {
    init { require(size in 1..ConnectionWire.MAX_PAYLOAD) }
    private val io = ProducerMeasurementIo()
    private val endpoint = ProducerMeasurementEndpoint()
    private val producer = LocalChannelProducer(ConnectionWire.MAX_PAYLOAD, 5000)
    var differentArrayBytes = 0L; private set
    init { advance() }
    private fun advance() = producer.advance(endpoint, endpoint.lease, ConnectionWire.CONTROL, io, 0)
    fun step() {
        val result = io.finish(size)
        val original = (result.request as ConnectionIoRequest.Read).loan.bytes
        producer.complete(result, false, 0)
        advance()
        val transferred = checkNotNull(endpoint.owned)
        if (transferred !== original) differentArrayBytes += size
        // New native admission must not target storage still owned by the endpoint.
        check(checkNotNull(io.pending).loan.bytes !== transferred)
        endpoint.consume(size)
    }
    override fun close() {
        io.stop()
        if (io.pending != null) producer.complete(io.finish(0), true, 0)
        check(io.retired())
        endpoint.discard()
    }
}

internal fun producerAllocationSample(size: Int, iterations: Int): JsonObject {
    require(iterations in 1..PRODUCER_ALLOCATION_ITERATIONS)
    ProducerAllocationTrial(size).use { trial ->
        val counters = NativeCounters()
        val thread = Thread.currentThread()
        val before = counters.read(thread)
        val allocatedBefore = checkNotNull(before.allocated)
        val start = System.nanoTime()
        repeat(iterations) { trial.step() }
        val elapsed = System.nanoTime() - start
        val after = counters.read(thread)
        return buildJsonObject {
            put("payload_bytes", size); put("iterations", iterations)
            put("offered_different_array_bytes", trial.differentArrayBytes)
            put("elapsed_ns", elapsed); put("driver_cpu_ns", after.cpu - before.cpu)
            put("driver_allocated_bytes", checkNotNull(after.allocated) - allocatedBefore)
        }
    }
}

fun main(arguments: Array<String>) {
    require(arguments.size == 1)
    val output = measurementOutput(arguments[0])
    val results = mutableListOf<JsonObject>()
    repeat(7) { trial ->
        val sizes = listOf(64, 4096, 16384, 65536)
        for (size in if (trial % 2 == 0) sizes else sizes.reversed()) {
            results.add(buildJsonObject {
                put("trial", trial); put("warmup", trial < 2)
                producerAllocationSample(size, PRODUCER_ALLOCATION_ITERATIONS).forEach { (key, value) -> put(key, value) }
            })
            persistMeasurement(output, results)
            println("Completed ${results.size}/28 producer allocation trials")
        }
    }
}
