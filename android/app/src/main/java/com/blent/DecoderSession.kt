package com.blent

import android.media.MediaCodec
import android.os.Handler
import android.os.HandlerThread
import android.util.Log
import android.view.Surface
import java.util.concurrent.atomic.AtomicLong
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.asStateFlow
import com.blent.VideoReceiver.Companion.ACK_EVERY
import com.blent.VideoReceiver.Companion.TAG

internal interface DecoderEvents {
    fun rendered(receipt: RenderAcknowledgement)
    fun invalidated()
}

/** Owns one codec and both output threads. The monitor is shared with the
 * receiver so Surface, generation and codec handoffs remain one transaction. */
internal class DecoderSession(
    private val monitor: Any,
    private val running: () -> Boolean,
    private val timing: FrameTiming,
    private val events: DecoderEvents,
    private val statistics: () -> ReceiverStatistics,
) {
    @Volatile var mediaCodec: MediaCodec? = null; private set
    @Volatile var configuredSelectionReceipt: String? = null; private set
    @Volatile private var codecAlive = false
    private var outputThread: Thread? = null
    private var frameCallbackThread: HandlerThread? = null
    var callbackThreadFactory: () -> HandlerThread = { HandlerThread("blent-frame-cb") }
    var createCodec: (DecoderFormat) -> MediaCodec = DecoderConfiguration::create
    var formatCodec: DecoderFormatFactory = { _, parameters, hints -> DecoderConfiguration.format(parameters, hints) }
    var outputClock: () -> Long = System::nanoTime
    private val renderedCount = AtomicLong(0)
    private val discardedCount = AtomicLong(0)
    val discardedOutputs: Long get() = discardedCount.get()
    private val outputWatchdog = DecoderOutputWatchdog()
    private var timingEpoch = timing.currentEpoch()
    private var lifetime: CodecLifetime? = null
    private var retiring: CodecLifetime? = null
    private var callbackDecoder: DecoderInputConsumer? = null
    var callbackFactory: DecoderCallbackFactory? = null
    var outputDriver = defaultDecoderOutputDriver()
    private val diagnosticState = MutableStateFlow(DecoderDiagnostics())
    val diagnostics = diagnosticState.asStateFlow()

    private class Startup(val epoch: FrameTiming.Epoch, val output: DecoderOutputDriver,
                          val format: DecoderFormatFactory, val callbacksFactory: DecoderCallbackFactory?, val hints: Boolean, val valid: () -> Boolean) {
        var owner: CodecLifetime? = null
        var thread: HandlerThread? = null
        var callbacks: DecoderInputConsumer? = null
        var decoderReceipt: String? = null
        var diagnostics: ActiveDecoderDiagnostics? = null
    }
    private var startup: Startup? = null // guarded by monitor

    /** Called on the receiver's I/O worker. Native setup never owns monitor. */
    fun setupCodec(surface: Surface, parameters: DecoderFormat, valid: () -> Boolean = { true }): Boolean {
        val attempt = reserveStartup(valid) ?: return false
        var published = false
        try {
            val codec = createCodec(parameters)
            attempt.owner = CodecLifetime(codec)
            if (!startupCurrent(attempt)) return false
            configureStartup(attempt, codec, surface, parameters)
            if (!startupCurrent(attempt)) return false
            codec.start()
            published = publishStartup(attempt)
            return published
        } catch (error: Exception) {
            Log.e(TAG, "Failed to setup codec", error)
            return false
        } finally {
            if (!published) discardStartup(attempt)
            CodecLifetime.finishStartup()
        }
    }

    private fun reserveStartup(valid: () -> Boolean): Startup? = synchronized(monitor) {
        if (!valid() || startup != null || mediaCodec != null || retiring?.finished == false) return null
        if (!CodecLifetime.beginStartup()) return null
        retiring = null
        outputWatchdog.restarted(outputClock())
        Startup(timing.beginEpoch(), outputDriver, formatCodec, callbackFactory, outputWatchdog.lowLatencyHints, valid).also { startup = it }
    }

    private fun startupCurrent(attempt: Startup): Boolean = synchronized(monitor) {
        startup === attempt && attempt.valid()
    }

    private fun configureStartup(attempt: Startup, codec: MediaCodec, surface: Surface, parameters: DecoderFormat) {
        val format = attempt.format(codec, parameters, attempt.hints)
        attempt.decoderReceipt = parameters.selection?.receipt(attempt.hints)
        attempt.diagnostics = DecoderDiagnosticProbe.capture(codec, parameters, format)
        if (!startupCurrent(attempt)) return
        val thread = callbackThreadFactory().also { attempt.thread = it; it.start() }
        val handler = Handler(thread.looper)
        val callbacks = callbacks(attempt.owner!!, handler, attempt.epoch, attempt.callbacksFactory)
        attempt.callbacks = callbacks
        callbacks?.install()
        codec.configure(format, surface, null, 0)
        if (!startupCurrent(attempt)) return
        codec.setVideoScalingMode(MediaCodec.VIDEO_SCALING_MODE_SCALE_TO_FIT)
        // Callback execution is acknowledgement timing, not physical presentation.
        codec.setOnFrameRenderedListener({ _, presentationTimeUs, _ ->
            notifyRendered(codec, attempt.epoch, presentationTimeUs.toInt())
        }, handler)
    }

    private fun publishStartup(attempt: Startup): Boolean = synchronized(monitor) {
        if (!startupCurrent(attempt)) return false
        val owner = attempt.owner!!
        discardedCount.set(0)
        timingEpoch = attempt.epoch
        configuredSelectionReceipt = attempt.decoderReceipt
        mediaCodec = owner.codec
        lifetime = owner
        callbackDecoder = attempt.callbacks
        frameCallbackThread = attempt.thread
        codecAlive = true
        if (attempt.callbacks == null) startOutputThread(owner.codec, attempt.epoch, attempt.output)
        diagnosticState.value = DecoderDiagnostics(attempt.diagnostics, !attempt.hints)
        startup = null
        Log.i(TAG, "Codec configured and started with surface")
        true
    }

    private fun discardStartup(attempt: Startup) {
        synchronized(monitor) {
            if (startup === attempt) startup = null
            timing.retire(attempt.epoch)
            if (mediaCodec === attempt.owner?.codec) {
                codecAlive = false
                mediaCodec = null
                lifetime = null
                callbackDecoder = null
                frameCallbackThread = null
            }
        }
        attempt.callbacks?.close()
        attempt.thread?.let {
            it.quitSafely()
            if (Thread.currentThread() !== it) it.join(500)
        }
        // Setup owns these locals until its native call returns. Retirement can
        // now free them; the process-wide gate remains closed until then.
        attempt.owner?.let { it.retire(); it.awaitRetirement(500) }
    }

    private fun notifyRendered(codec: MediaCodec, codecTiming: FrameTiming.Epoch, sequence: Int) {
        // Timing/log work can yield. Validate again at acknowledgement so a
        // callback that overlapped replacement cannot target the new session.
        val decodeMicros = timing.decodeMicrosFor(sequence, codecTiming)
        val receipt = synchronized(monitor) {
            if (mediaCodec !== codec || !codecAlive || !running()) return
            if (renderedCount.incrementAndGet() % ACK_EVERY != 0L) return
            RenderAcknowledgement(sequence, decodeMicros, configuredSelectionReceipt) {
                mediaCodec === codec && codecAlive && running()
            }
        }
        events.rendered(receipt)
    }

    private fun callbacks(owner: DecoderCodecAccess, handler: Handler, epoch: FrameTiming.Epoch,
                          factory: DecoderCallbackFactory?): DecoderInputConsumer? {
        if (factory == null) return null
        val capturedStatistics = statistics()
        return factory(owner, handler, object : DecoderInputEvents {
            override fun queued() { synchronized(monitor) { if (mediaCodec === owner.codec) checkOutputProgress() } }
            override fun output(sequence: Int) { recordOutput(owner.codec, epoch, capturedStatistics, sequence) }
            override fun failed(error: Exception) { retireFailedOutput(owner.codec, "Decoder callback failed", error) }
        })
    }

    private fun recordOutput(codec: MediaCodec, epoch: FrameTiming.Epoch, statistics: ReceiverStatistics, sequence: Int) {
        timing.noteReleased(sequence, epoch)
        val now = outputClock()
        synchronized(monitor) {
            if (mediaCodec !== codec || !codecAlive) return
            outputWatchdog.output(now)
            statistics.frameRendered()
        }
    }

    /**
     * Dedicated render thread: drains decoded frames and releases them to the
     * surface as soon as they're ready, independent of network reads. This
     * reduces coupling to network stalls; it does not impose a latency bound.
     */
    fun startOutputThread(codec: MediaCodec, codecTiming: FrameTiming.Epoch = timingEpoch,
                          driver: DecoderOutputDriver = outputDriver) {
        val outputStatistics = statistics()
        val outputOwner = synchronized(monitor) { ownerFor(codec) } ?: return
        val worker = Runnable {
            val info = MediaCodec.BufferInfo()
            val policy = outputPolicy(codec, outputOwner, driver) ?: return@Runnable
            var rendered = 0L
            while (codecAlive && mediaCodec === codec) {
                try {
                    val index = outputOwner.use { codec.dequeueOutputBuffer(info, 10_000) } ?: break
                    if (mediaCodec !== codec || !codecAlive || outputOwner.retired) break
                    if (index >= 0) {
                        val seq = outputOwner.use {
                            policy.release(index, info.presentationTimeUs.toInt())
                        } ?: break
                        recordOutput(codec, codecTiming, outputStatistics, seq)
                        rendered++
                        if (rendered <= 2) Log.i(TAG, "Rendered output frame #$rendered")
                    }
                } catch (e: IllegalStateException) {
                    retireFailedOutput(codec, "Output thread: codec gone", e)
                    break
                } catch (e: Exception) {
                    retireFailedOutput(codec, "Output thread error", e)
                    break
                }
            }
        }
        outputThread = driver.thread(worker).apply { start() }
    }

    private fun outputPolicy(codec: MediaCodec, owner: DecoderCodecAccess,
                             driver: DecoderOutputDriver): DecoderOutputPolicy? = try {
        driver.policy(codec, { mediaCodec === codec && codecAlive && !owner.retired },
            { discardedCount.incrementAndGet() })
    } catch (error: Exception) {
        retireFailedOutput(codec, "Output policy failed", error)
        null
    }

    private fun retireFailedOutput(codec: MediaCodec, message: String, error: Exception) {
        if (codecAlive) Log.w(TAG, message, error)
        synchronized(monitor) {
            if (mediaCodec === codec || startup?.owner?.codec === codec) resetCodec()
        }
    }

    /**
     * Queue one access unit into the decoder. Never silently drops frames:
     * a dropped P-frame corrupts the picture until the next keyframe. If no
     * input buffer frees up within ~200ms the codec is genuinely stuck and we
     * reset it instead.
     */
    fun feedDecoder(
        codec: MediaCodec, data: ByteArray, offset: Int, size: Int,
        isConfig: Boolean, presentationTimeUs: Long, arrivalNanos: Long = System.nanoTime()
    ) {
        useInput(codec) { access ->
            val input = DecoderInput(data, offset, size, isConfig, presentationTimeUs)
            access.arrived(input, arrivalNanos)
            access.offer(input)
        }
    }

    /** Captures identity under the receiver monitor, then runs native input
     * outside it. Late failures can retire only the original admitted decoder. */
    fun useInput(codec: MediaCodec, operation: (DecoderInputAccess) -> Boolean): Boolean {
        val access = synchronized(monitor) {
            val owner = ownerFor(codec) ?: return false
            val epoch = timingEpoch
            CodecInputAccess(owner, callbackDecoder,
                { sequence, nanos -> timing.noteArrival(sequence, epoch, nanos) },
                { synchronized(monitor) { if (mediaCodec === codec) checkOutputProgress() } })
        }
        return try { operation(access) }
        catch (error: Exception) {
            retireFailedOutput(codec, "Decoder input failed", error)
            false
        }
    }

    // Caller holds the receiver monitor. The lazy branch supports a codec
    // adopted by a platform/test adapter before starting its I/O workers.
    private fun ownerFor(codec: MediaCodec): CodecLifetime? {
        if (mediaCodec !== codec) return null
        if (lifetime?.codec !== codec) lifetime = CodecLifetime(codec)
        return lifetime?.takeUnless { it.retired }
    }

    fun checkOutputProgress() {
        val stall = outputWatchdog.queued(outputClock()) ?: return
        Log.w(
            TAG,
            "Decoder took ${stall.queued} frames and showed none for " +
                "${stall.silentNanos / 1_000_000} ms — restarting" +
                (if (stall.droppedHints) " without low-latency hints" else "")
        )
        // A fresh decoder needs config and a keyframe; reconnect obtains both.
        resetCodec()
    }

    fun retireOutput() { codecAlive = false; lifetime?.closeAdmission(); callbackDecoder?.close() }

    /** Observe the original native retirement without holding the receiver lock.
     * Expiry retains the owner and keeps replacement admission closed. */
    fun awaitRetirement(milliseconds: Long): Boolean {
        require(milliseconds in 1..60_000)
        val owner = synchronized(monitor) { retiring } ?: return true
        owner.awaitRetirement(milliseconds)
        return owner.finished
    }

    /** Tear the decoder down without touching the surface or the socket. */
    fun releaseCodec() {
        synchronized(monitor) {
            startup?.let { timing.retire(it.epoch) }
            startup = null
            codecAlive = false
            timing.retire(timingEpoch)
            val owner = mediaCodec?.let { ownerFor(it) } ?: lifetime
            mediaCodec = null
            diagnosticState.value = DecoderDiagnostics(watchdogFallback = !outputWatchdog.lowLatencyHints)
            lifetime = null
            callbackDecoder?.close()
            callbackDecoder = null
            outputThread = null
            frameCallbackThread?.quitSafely()
            frameCallbackThread = null
            if (owner != null) {
                retiring = owner
                owner.retire()
            }
        }
    }

    fun resetCodec() {
        synchronized(monitor) {
            // Every replacement decoder needs headers and a keyframe. Leave
            // creation to reconnect, which obtains both from the host.
            events.invalidated()
            releaseCodec()
        }
    }

}
