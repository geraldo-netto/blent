package com.blent.connection

import java.nio.file.Files
import kotlinx.serialization.json.*

private fun activeCues(owner: ActorOwner, count: Int, fps: Int): LongArray {
    val started = System.nanoTime()
    return LongArray(count) { index ->
        paceMeasurement(started, index, fps)
        owner.checkOwner()
        val latency = owner.probe.change(true, owner::checkOwner)
        if (index % 2048 == 2047) println("T1768 acknowledged ${index + 1}/$count original cues")
        latency
    }
}

internal fun activeActorSample(fps: Int, warmup: Int, events: Int): JsonObject {
    require(fps == 0 || fps == 60)
    require(warmup in 0..2000 && events in 1..10000)
    val origin = System.nanoTime()
    val clock = { (System.nanoTime() - origin) / 1_000_000 }
    ActorOwner("worker", 1000, true, clock, warmup + events).use { owner ->
        owner.start()
        activeCues(owner, warmup, fps)
        val counters = NativeCounters()
        val actorBefore = counters.read(owner.thread())
        val driverBefore = counters.read(Thread.currentThread())
        val advancesBefore = owner.probe.advances.get()
        val start = System.nanoTime()
        val latency = activeCues(owner, events, fps)
        val elapsed = System.nanoTime() - start
        val actorAfter = counters.read(owner.thread())
        val driverAfter = counters.read(Thread.currentThread())
        val advances = owner.probe.advances.get() - advancesBefore
        check(advances >= events)
        return buildJsonObject {
            put("fps", fps); put("events", events); put("warmup_events", warmup)
            put("advances", advances); put("elapsed_ns", elapsed)
            put("actor_cpu_ns", actorAfter.cpu - actorBefore.cpu)
            put("actor_allocated_bytes", checkNotNull(actorAfter.allocated) - checkNotNull(actorBefore.allocated))
            put("driver_cpu_ns", driverAfter.cpu - driverBefore.cpu)
            put("driver_allocated_bytes", checkNotNull(driverAfter.allocated) - checkNotNull(driverBefore.allocated))
            putJsonArray("latency_ns") { latency.forEach { add(it) } }
        }
    }
}

fun main(arguments: Array<String>) {
    require(arguments.size == 1)
    val output = measurementOutput(arguments.single()).toPath()
    val trials = mutableListOf<JsonObject>()
    repeat(6) { trial ->
        for (fps in if (trial % 2 == 0) listOf(0, 60) else listOf(60, 0)) {
            println("T1768 starting trial=${trials.size + 1}/12 fps=$fps")
            val result = activeActorSample(fps, if (fps == 0) 2000 else 32, if (fps == 0) 10000 else 120)
            trials.add(buildJsonObject {
                put("trial", trial); put("warmup", trial == 0)
                result.forEach { (key, value) -> put(key, value) }
            })
            Files.write(output, buildJsonObject {
                put("scope", "Hosted current worker actor; one original cue at a time, no Android native work")
                put("trials", JsonArray(trials))
            }.toString().toByteArray(Charsets.UTF_8))
            println("T1768 completed ${trials.size}/12 trials")
        }
    }
}
