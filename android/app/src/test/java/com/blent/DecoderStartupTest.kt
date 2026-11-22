package com.blent

import android.graphics.SurfaceTexture
import android.media.MediaCodec
import android.media.MediaCrypto
import android.media.MediaFormat
import android.os.Handler
import android.view.Surface
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicInteger
import java.util.concurrent.atomic.AtomicReference
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config
import org.robolectric.annotation.Implementation
import org.robolectric.annotation.Implements
import org.robolectric.shadows.ShadowMediaCodec

@Suppress("UNUSED_PARAMETER")
@Implements(MediaCodec::class)
class StartupCodecShadow : ShadowMediaCodec() {
    companion object {
        var stage = ""
        var entered = CountDownLatch(1)
        var resume = CountDownLatch(1)
        var released = CountDownLatch(1)
        var renderedListener: MediaCodec.OnFrameRenderedListener? = null
        fun gate(at: String) {
            if (stage != at) return
            entered.countDown()
            check(resume.await(5, TimeUnit.SECONDS))
        }
    }
    @Implementation fun configure(format: MediaFormat, surface: Surface?, crypto: MediaCrypto?, flags: Int) { gate("configure") }
    @Implementation fun setVideoScalingMode(mode: Int) {}
    @Implementation fun setOnFrameRenderedListener(listener: MediaCodec.OnFrameRenderedListener, handler: Handler) {
        renderedListener = listener
    }
    @Implementation fun start() { gate("start") }
    @Implementation fun stop() {}
    @Implementation fun release() { released.countDown() }
    @Implementation fun dequeueOutputBuffer(info: MediaCodec.BufferInfo, timeoutUs: Long): Int {
        Thread.sleep(10)
        return -1
    }
}

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [27, 34], shadows = [StartupCodecShadow::class])
class DecoderStartupTest {
    @org.junit.After fun awaitFixtureRetirement() = awaitDecoderRetirement()
    @Test fun t1747StartupRetainsItsOriginalFormatFactory() {
        StartupCodecShadow.stage = ""
        val surface = Surface(SurfaceTexture(1))
        val receiver = VideoReceiver { error("T1747 no transport") }
        org.robolectric.util.ReflectionHelpers.setField(receiver, "isRunning", true)
        var configured = 0
        receiver.decoder.formatCodec = { _, parameters, hints ->
            configured++
            DecoderConfiguration.format(parameters, hints)
        }
        receiver.decoder.createCodec = { parameters ->
            receiver.decoder.formatCodec = { _, _, _ -> error("T1747 replacement factory entered old startup") }
            MediaCodec.createDecoderByType(parameters.mimeType)
        }
        try {
            assertTrue(receiver.setupCodec(surface))
            assertEquals(1, configured)
        } finally { receiver.stop(); surface.release() }
    }

    @Test fun t1749StartupCapturesCallbackOwnerAndClosesFailedInstallation() {
        for (failInstall in listOf(false, true)) callbackStartup(failInstall)
    }

    private fun callbackStartup(failInstall: Boolean) {
        StartupCodecShadow.stage = ""
        val surface = Surface(SurfaceTexture(1))
        val receiver = VideoReceiver { error("T1749 no transport") }
        org.robolectric.util.ReflectionHelpers.setField(receiver, "isRunning", true)
        val calls = mutableListOf<String>()
        receiver.decoder.callbackFactory = { _, _, _ -> object : DecoderInputConsumer {
            override fun install() { calls.add("install"); check(!failInstall) { "T1749 install failure" } }
            override fun offer(input: DecoderInput) = error("T1749 no input")
            override fun close() { calls.add("close") }
        } }
        receiver.decoder.createCodec = { parameters ->
            receiver.decoder.callbackFactory = { _, _, _ -> error("T1749 replaced factory") }
            MediaCodec.createDecoderByType(parameters.mimeType)
        }
        try {
            assertEquals(!failInstall, receiver.setupCodec(surface))
            receiver.decoder.releaseCodec()
            awaitDecoderRetirement(receiver.decoder)
            assertEquals(listOf("install", "close"), calls)
        } finally { receiver.stop(); surface.release() }
    }

    @Test fun t1751StartupCapturesOriginalOutputAdapters() {
        StartupCodecShadow.stage = ""
        val surface = Surface(SurfaceTexture(1))
        val receiver = VideoReceiver { error("T1751 no transport") }
        org.robolectric.util.ReflectionHelpers.setField(receiver, "isRunning", true)
        val policyCreated = java.util.concurrent.CountDownLatch(1)
        val original = defaultDecoderOutputDriver()
        receiver.decoder.outputDriver = original.copy(policy = { codec, alive, discarded ->
            policyCreated.countDown()
            original.policy(codec, alive, discarded)
        })
        receiver.decoder.createCodec = { parameters ->
            receiver.decoder.outputDriver = original.copy(thread = { error("T1751 replaced thread factory") },
                policy = { _, _, _ -> error("T1751 replaced output policy") })
            MediaCodec.createDecoderByType(parameters.mimeType)
        }
        try {
            assertTrue(receiver.setupCodec(surface))
            assertTrue(policyCreated.await(2, java.util.concurrent.TimeUnit.SECONDS))
        } finally { receiver.stop(); surface.release() }
    }

    @Test fun t1751OutputPolicyFailureRetiresItsOriginalCodec() {
        StartupCodecShadow.stage = ""
        val surface = Surface(SurfaceTexture(1))
        val invalidated = java.util.concurrent.CountDownLatch(1)
        val decoder = DecoderSession(Any(), { true }, FrameTiming(), object : DecoderEvents {
            override fun rendered(receipt: RenderAcknowledgement) {}
            override fun invalidated() { invalidated.countDown() }
        }, { ReceiverStatistics() })
        decoder.createCodec = { MediaCodec.createDecoderByType(it.mimeType) }
        decoder.outputDriver = defaultDecoderOutputDriver().copy(policy = { _, _, _ -> error("T1751 policy failure") })
        try {
            assertTrue(decoder.setupCodec(surface, DecoderFormat("video/avc", 1280, 800, 60)))
            assertTrue("T1751 policy failure must invalidate the decoder", invalidated.await(2, java.util.concurrent.TimeUnit.SECONDS))
            decoder.releaseCodec()
            awaitDecoderRetirement(decoder)
            assertNull(decoder.mediaCodec)
        } finally { decoder.releaseCodec(); surface.release() }
    }

    @Test fun t497_factoryFailureRetiresStartupAndAcknowledgementsIgnoreRetiredCodec() {
        StartupCodecShadow.stage = ""
        StartupCodecShadow.renderedListener = null
        val surface = Surface(SurfaceTexture(1))
        val receiver = VideoReceiver { error("T497 uses only local codec callbacks") }
        val acknowledgements = mutableListOf<Int>()
        receiver.onFrameRendered = { acknowledgements.add(it.sequence) }
        org.robolectric.util.ReflectionHelpers.setField(receiver, "isRunning", true)
        try {
            receiver.decoder.createCodec = { throw IllegalStateException("T497 failed allocation") }
            assertFalse(receiver.setupCodec(surface))
            assertNull(receiver.decoder.mediaCodec)
            assertEquals(0L, receiver.decoder.discardedOutputs)
            receiver.decoder.createCodec = { MediaCodec.createDecoderByType(it.mimeType) }
            assertTrue("T497 factory failure retained process startup reservation", receiver.setupCodec(surface))
            val codec = receiver.decoder.mediaCodec!!
            val listener = StartupCodecShadow.renderedListener!!
            for (seq in 1..VideoReceiver.ACK_EVERY) listener.onFrameRendered(codec, seq.toLong(), 0L)
            assertEquals(listOf(VideoReceiver.ACK_EVERY), acknowledgements)
            receiver.stop()
            for (seq in 1..VideoReceiver.ACK_EVERY) listener.onFrameRendered(codec, seq.toLong(), 0L)
            assertEquals("T497 retired callback sent an ACK", 1, acknowledgements.size)
        } finally { receiver.stop(); surface.release() }
    }

    @Test fun t425_stopDuringNativeCreation() = blockedSetup("create", false)
    @Test fun t425_stopDuringNativeConfiguration() = blockedSetup("configure", false)
    @Test fun t425_stopDuringNativeStart() = blockedSetup("start", false)
    @Test fun t425_surfaceDestructionDuringNativeConfiguration() = blockedSetup("configure", true)

    @Test fun t425_surfaceCallbackDefersNativeSetup() {
        val surface = Surface(SurfaceTexture(1))
        val view = object : android.view.SurfaceView(androidx.test.core.app.ApplicationProvider.getApplicationContext()) {
            override fun getHolder(): android.view.SurfaceHolder = object : android.view.SurfaceHolder by super.getHolder() {
                override fun getSurface() = surface
            }
        }
        assertTrue("T425: fixture must exercise the ready-Surface path", view.holder.surface.isValid)
        val receiver = VideoReceiver()
        VideoReceiver::class.java.getDeclaredField("isRunning").apply { isAccessible = true }.set(receiver, true)
        val allocations = AtomicInteger()
        receiver.decoder.createCodec = { allocations.incrementAndGet(); throw IllegalStateException("native UI setup") }
        try {
            receiver.setSurface(view)
            assertEquals("T425: Activity Surface callback entered native setup", 0, allocations.get())
        } finally { receiver.stop(); surface.release() }
    }

    private fun blockedSetup(stage: String, destroySurface: Boolean) {
        StartupCodecShadow.stage = stage
        StartupCodecShadow.entered = CountDownLatch(1)
        StartupCodecShadow.resume = CountDownLatch(1)
        StartupCodecShadow.released = CountDownLatch(1)
        val surface = Surface(SurfaceTexture(1))
        val receiver = VideoReceiver()
        val result = AtomicReference<Boolean?>()
        val error = AtomicReference<Throwable?>()
        receiver.decoder.createCodec = { mime ->
            StartupCodecShadow.gate("create")
            MediaCodec.createDecoderByType(mime.mimeType)
        }
        val setup = Thread {
            try { result.set(receiver.setupCodec(surface)) }
            catch (failure: Throwable) { error.set(failure) }
        }.apply { start() }
        val stopped = CountDownLatch(1)
        val stop = Thread {
            if (destroySurface) receiver.onSurfaceDestroyed() else receiver.stop()
            stopped.countDown()
        }
        try {
            assertTrue("T425: native setup was not reached", StartupCodecShadow.entered.await(2, TimeUnit.SECONDS))
            stop.start()
            assertTrue("T425: native setup blocked lifecycle shutdown", stopped.await(300, TimeUnit.MILLISECONDS))
            assertNull(receiver.decoder.mediaCodec)
            assertNull("T417: unfinished/retired startup must not publish diagnostics", receiver.decoder.diagnostics.value.active)
            recreatedReceiverCannotAllocate(surface)
        } finally {
            StartupCodecShadow.resume.countDown()
            setup.join(2000)
            stop.join(2000)
            receiver.stop()
            surface.release()
        }
        error.get()?.let { throw AssertionError("T425: setup worker", it) }
        assertNull("T417: late startup diagnostics escaped retirement", receiver.decoder.diagnostics.value.active)
        assertFalse("T425: setup did not retire", setup.isAlive)
        assertEquals("T425: late setup published an obsolete codec", false, result.get())
        assertNull(receiver.decoder.mediaCodec)
        assertTrue("T425: late codec leaked", StartupCodecShadow.released.await(2, TimeUnit.SECONDS))
    }

    private fun recreatedReceiverCannotAllocate(surface: Surface) {
        val allocations = AtomicInteger()
        repeat(3) {
            val recreated = VideoReceiver()
            recreated.decoder.createCodec = {
                allocations.incrementAndGet()
                throw IllegalStateException("T425: replacement bypassed setup ownership")
            }
            try { assertFalse(recreated.setupCodec(surface)) }
            finally { recreated.stop() }
        }
        assertEquals("T425: Activity recreation admitted repeated native allocations", 0, allocations.get())
    }
}
