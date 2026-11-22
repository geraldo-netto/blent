package com.blent.connection

import java.nio.file.Files
import kotlinx.serialization.json.*

private data class RoleResult(
    val role: String, val elapsed: Long, val advances: Long, val cpu: Long, val allocated: Long?,
    val hinted: List<Long>, val missed: List<Long>,
)

private fun changes(owner: ActorOwner, hinted: Boolean, count: Int): List<Long> {
    require(count in 1..32)
    return List(count) {
        owner.checkOwner()
        val latency = owner.probe.change(hinted, owner::checkOwner)
        Thread.sleep(3)
        latency
    }
}

private fun finishResult(owner: ActorOwner, before: Counters, after: Counters, advances: Long, elapsed: Long): RoleResult {
    val allocated = before.allocated?.let { first -> after.allocated?.let { it - first } }
    return RoleResult(owner.role, elapsed, advances, after.cpu - before.cpu, allocated,
        changes(owner, true, 32), changes(owner, false, 2))
}

private fun roleJson(value: RoleResult): JsonObject = buildJsonObject {
    put("role", value.role); put("idle_elapsed_ns", value.elapsed); put("idle_advances", value.advances)
    put("idle_actor_cpu_ns", value.cpu)
    value.allocated?.let { put("idle_actor_allocated_bytes", it) }
    putJsonArray("hinted_latency_ns") { value.hinted.forEach { add(it) } }
    putJsonArray("missed_hint_latency_ns") { value.missed.forEach { add(it) } }
}

private fun trial(fallback: Long, known: Boolean, counters: NativeCounters, clock: () -> Long): JsonObject {
    ActorOwner("worker", fallback, known, clock, 64).use { worker ->
        ActorOwner("admission", fallback, known, clock, 64).use { admission ->
            worker.start(); admission.start(); Thread.sleep(1000)
            val workerBefore = counters.read(worker.thread()); val admissionBefore = counters.read(admission.thread())
            val workerTicks = worker.probe.advances.get(); val admissionTicks = admission.probe.advances.get()
            val processBefore = counters.processCpu()
            val began = System.nanoTime()
            Thread.sleep(3000)
            val elapsed = System.nanoTime() - began
            val processCpu = counters.processCpu() - processBefore
            val workerAfter = counters.read(worker.thread()); val admissionAfter = counters.read(admission.thread())
            val workerAdvances = worker.probe.advances.get() - workerTicks
            val admissionAdvances = admission.probe.advances.get() - admissionTicks
            val workerResult = finishResult(worker, workerBefore, workerAfter, workerAdvances, elapsed)
            val admissionResult = finishResult(admission, admissionBefore, admissionAfter, admissionAdvances, elapsed)
            println("T918 settled fallback=$fallback known=$known: worker idle advances=${workerResult.advances}, admission idle advances=${admissionResult.advances}")
            return buildJsonObject {
                put("fallback_ms", fallback); put("known", known)
                put("idle_process_cpu_ns", processCpu)
                putJsonArray("roles") { add(roleJson(workerResult)); add(roleJson(admissionResult)) }
            }
        }
    }
}

fun main(arguments: Array<String>) {
    require(arguments.size == 1)
    val output = measurementOutput(arguments.single()).toPath()
    val counters = NativeCounters()
    val origin = System.nanoTime()
    val clock = { (System.nanoTime() - origin) / 1_000_000 }
    val results = arrayListOf<JsonObject>()
    for (fallback in listOf(10L, 100L, 1000L, 1000L, 100L, 10L)) {
        println("T918 starting known fallback=$fallback trial=${results.size + 1}/7")
        results.add(trial(fallback, true, counters, clock))
    }
    println("T918 starting unsupported-capability fallback control")
    results.add(trial(1000, false, counters, clock))
    val report = buildJsonObject {
        put("scope", "Hosted JVM actor scheduling; synthetic idle native ownership; no Android hardware")
        put("java", System.getProperty("java.runtime.version")); put("vm", System.getProperty("java.vm.name"))
        put("os", System.getProperty("os.name")); put("arch", System.getProperty("os.arch"))
        put("warmup_ms", 1000); put("idle_ms", 3000); put("hinted_samples_per_role", 32); put("missed_samples_per_role", 2)
        put("trials", JsonArray(results))
    }
    Files.write(output, (Json { prettyPrint = true }.encodeToString(JsonObject.serializer(), report) + "\n").toByteArray(Charsets.UTF_8))
}
