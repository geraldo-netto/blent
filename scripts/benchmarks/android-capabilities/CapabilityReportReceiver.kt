package com.blent.benchmark

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.os.Build
import com.blent.DecoderCapabilities
import kotlinx.coroutines.runBlocking
import org.json.JSONArray
import org.json.JSONObject
import java.io.File
import java.util.concurrent.atomic.AtomicBoolean

/** Shell-only metadata benchmark in a separate package; no capture is started. */
class CapabilityReportReceiver : BroadcastReceiver() {
    companion object { private val active = AtomicBoolean() }
    override fun onReceive(context: Context, intent: Intent) {
        if (!active.compareAndSet(false, true)) return
        val completion = goAsync()
        Thread({
            try {
                val result = try { runBlocking { measure() } }
                    catch (error: Exception) { JSONObject().put("error", error.stackTraceToString()) }
                result.put("run_id", intent.getStringExtra("run_id"))
                File(context.filesDir, "capabilities.json").writeText(result.toString(2))
            } finally { active.set(false); completion.finish() }
        }, "blent-capability-benchmark").start()
    }

    private suspend fun measure(): JSONObject {
        val rows = JSONArray()
        for (format in listOf(intArrayOf(640, 480, 30), intArrayOf(1280, 800, 60), intArrayOf(1920, 1080, 60), intArrayOf(2560, 1600, 90))) {
            rows.put(measureFormat(format))
        }
        return JSONObject().put("sdk", Build.VERSION.SDK_INT).put("fingerprint", Build.FINGERPRINT).put("formats", rows)
    }

    private suspend fun measureFormat(format: IntArray): JSONObject {
        val coldStart = System.nanoTime()
        val report = DecoderCapabilities.report(format[0], format[1], format[2])
        val coldMicros = (System.nanoTime() - coldStart) / 1000.0
        repeat(5) { DecoderCapabilities.report(format[0], format[1], format[2]) }
        val samples = JSONArray()
        repeat(12) {
            val started = System.nanoTime()
            repeat(4) { DecoderCapabilities.report(format[0], format[1], format[2]) }
            samples.put((System.nanoTime() - started) / 4000.0)
        }
        return JSONObject().put("width", format[0]).put("height", format[1]).put("fps", format[2])
            .put("cold_us", coldMicros).put("warm_us_per_report", samples).put("report", report)
    }
}
