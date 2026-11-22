package com.blent.connection

import kotlinx.serialization.json.*

internal const val MUX_ALLOCATION_ITERATIONS = 200000

/** One serialized production mux, no device/native work. A payload's ownership
 * returns after its exact frame finishes, before this fixture reoffers it. */
internal class MuxAllocationTrial(private val data: Boolean) {
    private val mux = ConnectionMux(1, 1, 64, 1, 64)
    private val payload = ByteArray(64) { 7 }
    private var sequence = 0L
    init {
        mux.activate(ConnectionWire.DISPLAY, 1)
        repeat(2) { check(mux.next()?.header?.kind == ConnectionWire.CREDIT); mux.finish(true) }
        check(mux.next() == null)
    }
    fun step(): Long = if (data) send() else { check(mux.next() == null); 0 }
    private fun send(): Long {
        val credit = ConnectionWire.Header(ConnectionWire.CREDIT, ConnectionWire.DISPLAY, 1, 1, sequence++, 0, 64)
        check(!mux.receive(credit))
        check(mux.offer(ConnectionWire.DISPLAY, payload))
        val frame = checkNotNull(mux.next())
        check(frame.bytes === payload && frame.length == 64)
        check(frame.header.kind == ConnectionWire.DATA)
        mux.finish(true)
        return frame.length.toLong()
    }
}

internal fun muxAllocationSample(data: Boolean, iterations: Int): JsonObject {
    require(iterations in 1..MUX_ALLOCATION_ITERATIONS)
    val trial = MuxAllocationTrial(data)
    val counters = NativeCounters()
    val thread = Thread.currentThread()
    val before = counters.read(thread)
    val allocatedBefore = checkNotNull(before.allocated) { "T1765 allocation counters unavailable" }
    val start = System.nanoTime()
    var sent = 0L
    repeat(iterations) { sent += trial.step() }
    val elapsed = System.nanoTime() - start
    val after = counters.read(thread)
    check(sent == if (data) 64L * iterations else 0L)
    return buildJsonObject {
        put("mode", if (data) "data" else "idle"); put("iterations", iterations)
        put("payload_bytes_sent", sent); put("elapsed_ns", elapsed)
        put("driver_cpu_ns", after.cpu - before.cpu)
        put("driver_allocated_bytes", checkNotNull(after.allocated) - allocatedBefore)
    }
}

fun main(arguments: Array<String>) {
    require(arguments.size == 1)
    val output = measurementOutput(arguments[0])
    val results = mutableListOf<JsonObject>()
    repeat(7) { trial ->
        for (data in if (trial % 2 == 0) listOf(false, true) else listOf(true, false)) {
            results.add(buildJsonObject {
                put("trial", trial); put("warmup", trial < 2)
                muxAllocationSample(data, MUX_ALLOCATION_ITERATIONS).forEach { (key, value) -> put(key, value) }
            })
            persistMeasurement(output, results)
            println("Completed ${results.size}/14 mux allocation trials")
        }
    }
}
