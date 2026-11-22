package com.blent.connection

import java.io.File
import java.util.concurrent.locks.LockSupport
import kotlinx.serialization.json.*

internal const val MEASUREMENT_FRAMES = 60

internal fun measurementCpu(counters: NativeCounters, threads: List<Thread>): Long =
    (threads + Thread.currentThread()).sumOf { counters.read(it).cpu }

internal fun paceMeasurement(start: Long, frame: Int, fps: Int) {
    if (fps == 0) return
    val due = start + frame * (1_000_000_000L / fps)
    while (true) {
        val remaining = due - System.nanoTime()
        if (remaining <= 0) return
        LockSupport.parkNanos(remaining)
        check(!Thread.interrupted()) { "Measurement cancelled" }
    }
}

internal fun measurementResult(size: Int, fps: Int, combined: Boolean, elapsed: Long, cpu: Long,
                               io: MeasuredIo, latency: LongArray, extra: JsonObject): JsonObject = buildJsonObject {
    put("payload_bytes", size); put("fps", fps); put("combined", combined); put("frames", latency.size)
    put("elapsed_ns", elapsed); put("driver_and_io_cpu_ns", cpu)
    put("native_completions_including_ack", io.completions)
    put("driver_poll_calls", io.polls); put("driver_wait_calls", io.waits); put("ready_hints_observed", io.readyHints)
    put("latency_ns", JsonArray(latency.map { JsonPrimitive(it) }))
    extra.forEach { (key, value) -> put(key, value) }
}

private fun persistMeasurement(output: File, results: List<JsonObject>) {
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

internal fun runMeasurements(output: File, sample: (Int, Int, Boolean) -> JsonObject) {
    val results = mutableListOf<JsonObject>()
    repeat(4) { trial ->
        for (fps in listOf(0, 60)) for (size in listOf(64, 4096, 16384, 65536)) {
            val order = if (trial % 2 == 0) listOf(false, true) else listOf(true, false)
            for (combined in order) {
                results.add(buildJsonObject {
                    put("warmup", trial == 0); put("trial", trial)
                    sample(size, fps, combined).forEach { (key, value) -> put(key, value) }
                })
                persistMeasurement(output, results)
                println("Completed ${results.size}/64 trials; payload=$size fps=$fps combined=$combined")
            }
        }
    }
}
