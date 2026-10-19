package com.blent.connection

import java.nio.file.Files
import java.nio.file.Path
import java.util.concurrent.atomic.AtomicReference
import kotlinx.serialization.json.*

/** Four finite single-thread probes; one escaping snapshot maximum, cleared on every exit. */
private fun deadlineWork(count: Int, combine: Boolean, escape: AtomicReference<ConnectionDeadline>?): Long {
    var checksum = 0L
    repeat(count) { index ->
        val value = ConnectionDeadline.at(null)
        val result = if (combine) value.combine(ConnectionDeadline.IDLE) else value
        escape?.set(result)
        check(result.atMs == null && result.known)
        checksum += index
    }
    return checksum
}

private fun allocationTrial(combine: Boolean, escaping: Boolean, counters: NativeCounters): JsonObject {
    val escape = if (escaping) AtomicReference<ConnectionDeadline>() else null
    try {
        val warmup = deadlineWork(1_000_000, combine, escape)
        check(warmup == 499_999_500_000L)
        val before = counters.read(Thread.currentThread())
        val checksum = deadlineWork(2_000_000, combine, escape)
        val after = counters.read(Thread.currentThread())
        check(checksum == 1_999_999_000_000L)
        return buildJsonObject {
            put("combine", combine); put("escaping", escaping); put("iterations", 2_000_000)
            put("cpu_ns", after.cpu - before.cpu)
            before.allocated?.let { first -> after.allocated?.let { put("allocated_bytes", it - first) } }
            put("checksum", checksum)
        }
    } finally { escape?.set(null) }
}

fun main(arguments: Array<String>) {
    require(arguments.size == 1)
    val counters = NativeCounters()
    val results = buildJsonArray {
        for (combine in listOf(false, true)) for (escaping in listOf(false, true)) {
            add(allocationTrial(combine, escaping, counters))
            println("T918 settled allocation probe combine=$combine escaping=$escaping")
        }
    }
    val report = buildJsonObject {
        put("scope", "Hosted JDK scalar replacement versus deliberate snapshot escape; no Android allocation claim")
        put("java", System.getProperty("java.runtime.version")); put("warmup_iterations", 1_000_000)
        put("trials", results)
    }
    val output = Path.of(arguments.single())
    Files.createDirectories(output.parent)
    Files.write(output, (Json { prettyPrint = true }.encodeToString(JsonObject.serializer(), report) + "\n").toByteArray(Charsets.UTF_8))
}
