package com.blent.benchmark

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.os.Build
import android.os.Debug
import com.blent.*
import org.json.JSONArray
import org.json.JSONObject
import java.io.File
import java.util.concurrent.atomic.AtomicBoolean

/** CPU/ART allocation probe only: never opens a microphone or speaker. */
class AudioQueueReceiver : BroadcastReceiver() {
    companion object { private val active = AtomicBoolean() }
    override fun onReceive(context: Context, intent: Intent) {
        if (!active.compareAndSet(false, true)) return
        val completion = goAsync()
        Thread({
            try {
                val result = try { measure() } catch (error: Exception) { JSONObject().put("error", error.stackTraceToString()) }
                result.put("run_id", intent.getStringExtra("run_id"))
                File(context.filesDir, "audio-queue.json").writeText(result.toString(2))
            } finally { active.set(false); completion.finish() }
        }, "blent-audio-probe").start()
    }

    private fun allocated() = Debug.getRuntimeStat("art.gc.bytes-allocated")?.toLongOrNull()
    private fun measure(): JSONObject {
        val rows = JSONArray()
        repeat(3) { queue(1000) }
        repeat(8) {
            val before = allocated()
            val cpu = Debug.threadCpuTimeNanos()
            val started = System.nanoTime()
            val checksum = queue(2000)
            val elapsed = System.nanoTime() - started
            val usedCpu = Debug.threadCpuTimeNanos() - cpu
            val after = allocated()
            rows.put(JSONObject().put("count", 2000).put("elapsed_ns", elapsed).put("cpu_ns", usedCpu)
                .put("allocated_bytes", if (before != null && after != null) after - before else JSONObject.NULL)
                .put("checksum", checksum))
        }
        val frequencies = JSONArray()
        for (ppm in listOf(-1000, -500, 0, 500, 1000)) frequencies.put(frequency(ppm))
        return JSONObject().put("sdk", Build.VERSION.SDK_INT).put("fingerprint", Build.FINGERPRINT)
            .put("identity_trials", rows).put("synthetic_frequencies", frequencies)
    }

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

    private fun frequency(ppm: Int): JSONObject {
        var now = 0L
        val queue = AudioPlaybackQueue(40) { now }
        val inputs = Array(23) { seed -> ShortArray(960) { (it * 137 + seed * 7919).toShort() } }
        var reused = 0
        var outputs = 0
        var checksum = 0L
        repeat(1200) { tick ->
            now = tick * 10L
            val frames = tick * 480L
            queue.offer(AudioBlock(inputs[tick % inputs.size], clock = AudioClockSample(1,
                frames + frames * ppm / 1_000_000, 1 + now * 1_000_000)))
            queue.nativeClock(AudioClockSample(2, frames, 900_000_000_001 + now * 1_000_000))
            queue.poll()?.let { chunk ->
                outputs++
                if (inputs.any { it === chunk.samples }) reused++
                for (sample in chunk.samples) checksum = checksum * 31 + sample
                checksum = checksum * 31 + chunk.at
                checksum = checksum * 31 + if (chunk.discontinuity) 1 else 0
            }
        }
        return JSONObject().put("ppm", ppm).put("outputs", outputs).put("transferred_arrays", reused).put("checksum", checksum)
    }
}
