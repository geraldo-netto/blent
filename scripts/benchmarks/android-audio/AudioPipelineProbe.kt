package com.blent.benchmark

import android.content.Context
import android.media.AudioAttributes
import android.os.Build
import android.os.Debug
import android.os.SystemClock
import com.blent.*
import kotlinx.coroutines.runBlocking
import okio.Buffer
import org.json.JSONArray
import org.json.JSONObject

internal object AudioPipelineProbe {
    private fun allocated() = Debug.getRuntimeStat("art.gc.bytes-allocated")!!.toLong()
    private val focus = object : OutputFocus { override fun paused() = false; override fun close() {} }

    fun measure(): JSONObject = runBlocking {
        val rows = JSONArray()
        for (mode in listOf("packet", "direct-parser-trial", "output")) {
            repeat(8) {
                val count = 500
                val payload = ByteArray(1920) { (it * 71).toByte() }
                val source = Buffer()
                repeat(count) { n -> source.writeLong(7).writeLong(n.toLong()).writeLong(n.toLong())
                    .writeShort(1920).writeByte(2).writeByte(0).write(payload) }
                val reader = AudioPackets(7, 2)
                val direct = DirectAudioPackets(7, 2)
                val track = PipelineTrack()
                val output = AudioOutput({ track }, focus) { 0 }
                val chunk = PlaybackChunk(ShortArray(960) { (it * 71).toShort() }, 0, false)
                val bytes = allocated()
                val cpu = Debug.threadCpuTimeNanos()
                val start = System.nanoTime()
                var checksum = 0L
                repeat(count) {
                    when (mode) {
                        "packet" -> checksum += reader.read(source).samples[0]
                        "direct-parser-trial" -> checksum += direct.read(source).samples[0]
                        else -> check(output.write(chunk) == SpeakerWrite.Written)
                    }
                }
                val elapsed = System.nanoTime() - start
                val usedCpu = Debug.threadCpuTimeNanos() - cpu
                val usedBytes = allocated() - bytes
                rows.put(JSONObject().put("mode", mode).put("count", count).put("elapsed_ns", elapsed)
                    .put("cpu_ns", usedCpu).put("allocated_bytes", usedBytes).put("checksum", checksum + track.checksum))
                output.close()
            }
        }
        JSONObject().put("sdk", Build.VERSION.SDK_INT).put("fingerprint", Build.FINGERPRINT).put("pipeline_trials", rows)
    }

    /** Silent native AudioTrack only; no microphone permission or capture. */
    fun native(context: Context): JSONObject = runBlocking {
        val native = AndroidSpeakerTrack(context, AudioAttributes.Builder().setUsage(AudioAttributes.USAGE_MEDIA).build(),
            AudioPreferences(gain = 0, builtIn = false))
        val output = AudioOutput({ native }, focus, SystemClock::elapsedRealtime)
        try {
            output.start()
            val samples = ShortArray(960)
            val before = native.description()
            val bytes = allocated()
            val cpu = Debug.threadCpuTimeNanos()
            val start = System.nanoTime()
            repeat(100) {
                check(output.write(PlaybackChunk(samples, SystemClock.elapsedRealtime(), false)) == SpeakerWrite.Written)
            }
            val elapsed = System.nanoTime() - start
            val usedCpu = Debug.threadCpuTimeNanos() - cpu
            val usedBytes = allocated() - bytes
            JSONObject().put("sdk", Build.VERSION.SDK_INT).put("fingerprint", Build.FINGERPRINT)
                .put("count", 100).put("elapsed_ns", elapsed).put("cpu_ns", usedCpu).put("allocated_bytes", usedBytes)
                .put("before", before).put("after", native.description())
        } finally { output.close() }
    }
}
