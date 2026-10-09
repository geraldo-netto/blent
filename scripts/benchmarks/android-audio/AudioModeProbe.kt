package com.blent.benchmark

import android.media.*
import android.os.Build
import android.os.Debug
import org.json.JSONArray
import org.json.JSONObject

/** Muted comparison with identical format and requested capacity. No capture. */
internal object AudioModeProbe {
    fun measure(reverse: Boolean): JSONObject {
        val rows = JSONArray()
        val modes = listOf(AudioTrack.PERFORMANCE_MODE_NONE, AudioTrack.PERFORMANCE_MODE_LOW_LATENCY)
        for (mode in if (reverse) modes.reversed() else modes) rows.put(run(mode))
        return JSONObject().put("sdk", Build.VERSION.SDK_INT).put("fingerprint", Build.FINGERPRINT).put("modes", rows)
    }

    private fun run(mode: Int): JSONObject {
        val minimum = AudioTrack.getMinBufferSize(48000, AudioFormat.CHANNEL_OUT_STEREO, AudioFormat.ENCODING_PCM_16BIT)
        check(minimum > 0)
        val track = AudioTrack.Builder().setAudioAttributes(AudioAttributes.Builder().setUsage(AudioAttributes.USAGE_MEDIA).build())
            .setAudioFormat(AudioFormat.Builder().setSampleRate(48000).setEncoding(AudioFormat.ENCODING_PCM_16BIT)
                .setChannelMask(AudioFormat.CHANNEL_OUT_STEREO).build())
            .setTransferMode(AudioTrack.MODE_STREAM).setBufferSizeInBytes(maxOf(minimum, 7680)).setPerformanceMode(mode).build()
        try {
            check(track.state == AudioTrack.STATE_INITIALIZED)
            check(track.setVolume(0f) == AudioTrack.SUCCESS)
            track.play()
            val samples = ShortArray(960)
            var zeros = 0
            var writes = 0
            var written = 0
            val cpu = Debug.threadCpuTimeNanos()
            val start = System.nanoTime()
            while (written < 96000) {
                check(System.nanoTime() - start < 3_000_000_000L)
                val count = track.write(samples, 0, minOf(samples.size, 96000 - written), AudioTrack.WRITE_NON_BLOCKING)
                check(count >= 0 && count % 2 == 0)
                writes++
                written += count
                if (count == 0) { zeros++; Thread.sleep(2) }
            }
            val elapsed = System.nanoTime() - start
            val usedCpu = Debug.threadCpuTimeNanos() - cpu
            val stamp = AudioTimestamp()
            val available = track.getTimestamp(stamp)
            val timestampAge = (System.nanoTime() - stamp.nanoTime) / 1_000_000.0
            return JSONObject().put("requested_mode", mode).put("granted_mode", track.performanceMode)
                .put("minimum_bytes", minimum).put("buffer_frames", track.bufferSizeInFrames)
                .put("underruns", track.underrunCount).put("write_calls", writes).put("zero_writes", zeros)
                .put("elapsed_ns", elapsed).put("cpu_ns", usedCpu)
                .put("timestamp_available", available).put("timestamp_frame", if (available) stamp.framePosition else JSONObject.NULL)
                .put("written_frames", written / 2)
                .put("queue_ms_from_counter", if (available) (written / 2 - stamp.framePosition) / 48.0 else JSONObject.NULL)
                .put("timestamp_age_ms", if (available) timestampAge else JSONObject.NULL)
                .put("estimated_queue_ms", if (available) (written / 2 - stamp.framePosition) / 48.0 - timestampAge else JSONObject.NULL)
        } finally { try { track.pause(); track.flush() } finally { track.release() } }
    }
}
