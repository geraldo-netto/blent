package com.blent.benchmark

import android.content.*
import android.media.*
import android.opengl.*
import android.os.*
import android.view.Surface
import org.json.*
import java.io.File
import java.util.concurrent.atomic.AtomicBoolean

/** Synthetic GL frames only. No camera permission, capture, network or production app access. */
class CameraProbeReceiver : BroadcastReceiver() {
    companion object { private val active = AtomicBoolean() }
    override fun onReceive(context: Context, intent: Intent) {
        if (!active.compareAndSet(false, true)) return
        val pending = goAsync()
        Thread {
            try {
                val result = try { measure(intent.getBooleanExtra("reverse", false)) }
                    catch (error: Throwable) { JSONObject().put("error", error.stackTraceToString()) }
                result.put("run_id", intent.getStringExtra("run_id"))
                File(context.filesDir, "camera.json").writeText(result.toString(2))
            } finally { active.set(false); pending.finish() }
        }.start()
    }
    private fun format(width: Int, height: Int) = MediaFormat.createVideoFormat("video/avc", width, height).apply {
        setInteger(MediaFormat.KEY_COLOR_FORMAT, MediaCodecInfo.CodecCapabilities.COLOR_FormatSurface)
        setInteger(MediaFormat.KEY_BIT_RATE, 4_000_000)
        setInteger(MediaFormat.KEY_FRAME_RATE, 30)
        setInteger(MediaFormat.KEY_I_FRAME_INTERVAL, 1)
        setInteger(MediaFormat.KEY_PROFILE, MediaCodecInfo.CodecProfileLevel.AVCProfileBaseline)
    }
    private fun measure(reverse: Boolean): JSONObject {
        val rows = JSONArray()
        for ((width, height) in listOf(640 to 480, 1280 to 720, 1920 to 1080)) {
            for (explicit in if (reverse) listOf(true, false) else listOf(false, true))
                rows.put(run(width, height, explicit))
        }
        return JSONObject().put("fingerprint", Build.FINGERPRINT).put("rows", rows)
    }
    private fun candidate(format: MediaFormat, explicit: Boolean): String? {
        if (!explicit) return null
        return MediaCodecList(MediaCodecList.REGULAR_CODECS).codecInfos.firstOrNull {
            it.isEncoder && it.isHardwareAccelerated && it.supportedTypes.contains("video/avc") &&
                runCatching { it.getCapabilitiesForType("video/avc").isFormatSupported(format) }.getOrDefault(false)
        }?.name
    }
    private fun run(width: Int, height: Int, explicit: Boolean): JSONObject {
        val memoryBefore = Debug.MemoryInfo().also { Debug.getMemoryInfo(it) }.totalPss
        val cpu = Debug.threadCpuTimeNanos()
        val started = System.nanoTime()
        val format = format(width, height)
        val candidate = candidate(format, explicit)
        val selectedAt = System.nanoTime()
        val codec = if (candidate == null) MediaCodec.createEncoderByType("video/avc") else MediaCodec.createByCodecName(candidate)
        var surface: Surface? = null
        var gl: EncoderGl? = null
        try {
            codec.configure(format, null, null, MediaCodec.CONFIGURE_FLAG_ENCODE)
            surface = codec.createInputSurface()
            codec.start()
            gl = EncoderGl(surface)
            val ready = System.nanoTime()
            val ages = ArrayList<Long>()
            val info = MediaCodec.BufferInfo()
            var bytes = 0L
            repeat(15) { frame ->
                val submitted = System.nanoTime()
                gl.draw(frame, submitted)
                val deadline = submitted + 1_000_000_000L
                var done = false
                while (!done) {
                    check(System.nanoTime() < deadline) { "Encoder output deadline" }
                    val index = codec.dequeueOutputBuffer(info, 10_000)
                    if (index >= 0) {
                        try {
                            if (info.size > 0 && info.flags and MediaCodec.BUFFER_FLAG_CODEC_CONFIG == 0) {
                                bytes += info.size
                                ages.add(System.nanoTime() - submitted)
                                done = true
                            }
                        } finally { codec.releaseOutputBuffer(index, false) }
                    }
                }
            }
            val elapsed = System.nanoTime() - started
            val usedCpu = Debug.threadCpuTimeNanos() - cpu
            val memory = Debug.MemoryInfo().also { Debug.getMemoryInfo(it) }.totalPss
            return JSONObject().put("width", width).put("height", height).put("explicit", explicit)
                .put("candidate", candidate ?: JSONObject.NULL).put("selected", codec.name)
                .put("hardware", codec.codecInfo.isHardwareAccelerated).put("selection_ns", selectedAt - started)
                .put("startup_ns", ready - started).put("elapsed_ns", elapsed).put("cpu_ns", usedCpu)
                .put("pss_before_kb", memoryBefore).put("pss_active_kb", memory).put("encoded_bytes", bytes)
                .put("delivery_ns", JSONArray(ages))
        } finally {
            gl?.close(); surface?.release()
            try { codec.stop() } finally { codec.release() }
        }
    }
}

private class EncoderGl(surface: Surface) : AutoCloseable {
    private val display = EGL14.eglGetDisplay(EGL14.EGL_DEFAULT_DISPLAY)
    private val context: EGLContext
    private val window: EGLSurface
    init {
        val versions = IntArray(2)
        check(EGL14.eglInitialize(display, versions, 0, versions, 1))
        val configs = arrayOfNulls<EGLConfig>(1)
        val count = IntArray(1)
        val attributes = intArrayOf(EGL14.EGL_RED_SIZE, 8, EGL14.EGL_GREEN_SIZE, 8, EGL14.EGL_BLUE_SIZE, 8,
            EGL14.EGL_RENDERABLE_TYPE, EGL14.EGL_OPENGL_ES2_BIT, 0x3142, 1, EGL14.EGL_NONE)
        check(EGL14.eglChooseConfig(display, attributes, 0, configs, 0, 1, count, 0) && count[0] > 0)
        context = EGL14.eglCreateContext(display, configs[0], EGL14.EGL_NO_CONTEXT,
            intArrayOf(EGL14.EGL_CONTEXT_CLIENT_VERSION, 2, EGL14.EGL_NONE), 0)
        window = EGL14.eglCreateWindowSurface(display, configs[0], surface, intArrayOf(EGL14.EGL_NONE), 0)
        check(EGL14.eglMakeCurrent(display, window, window, context))
    }
    fun draw(frame: Int, nanos: Long) {
        GLES20.glClearColor((frame % 3) / 2f, (frame % 5) / 4f, 0.25f, 1f)
        GLES20.glClear(GLES20.GL_COLOR_BUFFER_BIT)
        check(EGLExt.eglPresentationTimeANDROID(display, window, nanos))
        check(EGL14.eglSwapBuffers(display, window))
    }
    override fun close() {
        EGL14.eglMakeCurrent(display, EGL14.EGL_NO_SURFACE, EGL14.EGL_NO_SURFACE, EGL14.EGL_NO_CONTEXT)
        EGL14.eglDestroySurface(display, window); EGL14.eglDestroyContext(display, context)
        EGL14.eglTerminate(display); EGL14.eglReleaseThread()
    }
}
