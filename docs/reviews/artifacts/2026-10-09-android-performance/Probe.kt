package com.blent

import java.lang.management.ManagementFactory
import java.util.concurrent.CountDownLatch

internal data class AudioBlock(val samples: ShortArray, val discontinuity: Boolean = false, val clock: AudioClockSample? = null)
internal data class PlaybackChunk(val samples: ShortArray, val at: Long, val discontinuity: Boolean)
private var checksum = 0L

private fun queue(count: Int): Long {
    var now = 0L
    val queue = AudioPlaybackQueue(40) { now }
    val inputs = Array(23) { seed -> AudioBlock(ShortArray(960) { (it * 137 + seed * 7919).toShort() }) }
    repeat(3) { queue.offer(inputs[it]) }
    var sum = 0L
    repeat(count) { index ->
        now += 10
        queue.offer(inputs[(index + 3) % inputs.size])
        val chunk = checkNotNull(queue.poll())
        sum += chunk.samples[index % 960] + chunk.at + if (chunk.discontinuity) 1 else 0
    }
    return sum
}

private fun drift(count: Int): Long {
    val drift = AudioClockDrift()
    drift.source.observe(AudioClockSample(1, 0, 1), 0)
    drift.destination.observe(AudioClockSample(1, 0, 1), 0)
    drift.source.observe(AudioClockSample(1, 192096, 4_000_000_001), 4000)
    drift.destination.observe(AudioClockSample(1, 192000, 4_000_000_001), 4000)
    var sum = 0L
    repeat(count) { sum += checkNotNull(drift.correction(4000L + it % 2000)) }
    check(drift.correction(7001) == null)
    drift.clear()
    check(drift.correction(4000) == null)
    return sum
}

// Deterministic source/candidate output check, including nonidentity rates,
// resets, overflow, queue ages, partial-block phase, and changing windows.
private fun equivalence(): Long {
    var sum = 0L
    for (ppm in listOf(-1000, -500, 0, 500, 1000)) {
        var now = 0L
        val queue = AudioPlaybackQueue(40) { now }
        val random = java.util.Random(795)
        repeat(1200) { tick ->
            now = tick * 10L
            val epoch = 1L + tick / 400
            val sample = AudioClockSample(epoch, tick * 480L + tick * 480L * ppm / 1_000_000, 1 + now * 1_000_000)
            queue.offer(AudioBlock(ShortArray(960) { random.nextInt(65536).toShort() }, tick % 397 == 0, sample))
            queue.nativeClock(if (tick % 151 == 0) null else AudioClockSample(epoch, tick * 480L, sample.nanos))
            if (tick % 31 < 25) queue.poll()?.let { chunk ->
                for (value in chunk.samples) sum = sum * 31 + value
                sum = sum * 31 + chunk.at
                sum = sum * 31 + if (chunk.discontinuity) 1 else 0
            }
        }
        now += 201
        check(queue.poll() == null)
    }
    return sum
}

private fun retirement() {
    val owner = CodecLifetime(android.media.MediaCodec())
    val entered = CountDownLatch(1)
    val release = CountDownLatch(1)
    val user = Thread { owner.use { entered.countDown(); release.await() } }
    user.start()
    entered.await()
    owner.retire()
    val began = System.nanoTime()
    owner.awaitRetirement(500)
    val elapsed = System.nanoTime() - began
    check(!owner.finished && CodecLifetime.retirementPending())
    release.countDown()
    user.join(2000)
    owner.awaitRetirement(2000)
    check(owner.finished && !CodecLifetime.retirementPending())
    println("retirement\t$elapsed")
}

fun main(args: Array<String>) {
    val mode = args[0]
    if (mode == "retirement") { retirement(); return }
    if (mode == "equivalence") { println("equivalence\t${equivalence()}"); return }
    val count = args[1].toInt()
    val run: (Int) -> Long = if (mode == "queue") ::queue else ::drift
    repeat(8) { checksum = checksum xor run(count / 4) }
    val bean = ManagementFactory.getThreadMXBean() as com.sun.management.ThreadMXBean
    bean.isThreadAllocatedMemoryEnabled = true
    val id = Thread.currentThread().id
    val allocated = bean.getThreadAllocatedBytes(id)
    val cpu = bean.currentThreadCpuTime
    val began = System.nanoTime()
    val result = run(count)
    val elapsed = System.nanoTime() - began
    val usedCpu = bean.currentThreadCpuTime - cpu
    val bytes = bean.getThreadAllocatedBytes(id) - allocated
    checksum = checksum xor result
    println("$mode\t$count\t$elapsed\t$usedCpu\t$bytes\t$result\t$checksum")
}
