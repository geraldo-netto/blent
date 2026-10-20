package com.blent

import android.media.MediaCodec
import java.math.BigInteger
import java.util.concurrent.CopyOnWriteArrayList
import android.graphics.SurfaceTexture
import android.view.Surface
import kotlinx.coroutines.*
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config
import org.robolectric.util.ReflectionHelpers
import java.net.Socket
import java.net.SocketAddress
import java.io.InputStream
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicLong

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [27, 34], shadows = [StartupCodecShadow::class])
class VideoGenerationTest {
    @get:org.junit.Rule val decoderInventory = DecoderInventoryRule()
    @org.junit.After fun awaitNativeRetirement() = awaitDecoderRetirement()
    private fun generation(receiver: VideoReceiver): AtomicLong = ReflectionHelpers.getField(receiver, "sessionGeneration")
    private fun job(receiver: VideoReceiver): Job? = ReflectionHelpers.getField(receiver, "job")
    private fun current(receiver: VideoReceiver, value: Long): Boolean =
        VideoReceiver::class.java.getDeclaredMethod("isCurrent", Long::class.javaPrimitiveType)
            .apply { isAccessible = true }.invoke(receiver, value) as Boolean

    @Test fun t1181TerminalStartRejectsBeforeJobsSocketsOrDecoderAdmission() = runBlocking {
        var opens = 0
        val receiver = VideoReceiver(initialGeneration = Long.MAX_VALUE) { opens++; error("Unexpected socket") }
        try {
            repeat(3) {
                assertThrows(IllegalStateException::class.java) { receiver.start() }
                assertNull(job(receiver))
                assertEquals(Long.MAX_VALUE, generation(receiver).get())
                assertFalse(current(receiver, Long.MAX_VALUE))
                receiver.stop()
            }
            assertEquals(0, opens)
            assertNull(receiver.decoder.mediaCodec)
        } finally {
            val original = job(receiver)
            receiver.stop(); original?.join()
        }
    }

    @Test fun t1181LastAdmissionStopsAndCannotReviveOldTimerOrGeneration() = runBlocking {
        val receiver = VideoReceiver(initialGeneration = Long.MAX_VALUE - 1) { error("No Surface") }
        var original: Job? = null
        try {
            receiver.start()
            original = checkNotNull(job(receiver))
            assertEquals(Long.MAX_VALUE, generation(receiver).get())
            assertTrue(current(receiver, Long.MAX_VALUE))
            receiver.start(); assertSame(original, job(receiver))
            val statistics: ReceiverStatistics = ReflectionHelpers.getField(receiver, "statistics")
            receiver.stop(); original.join()
            assertTrue(original.isCompleted)
            assertEquals(Long.MAX_VALUE, generation(receiver).get())
            assertFalse(current(receiver, Long.MAX_VALUE))
            val now = System.nanoTime()
            statistics.sample(now)
            statistics.frameRendered(); statistics.bytesReceived(125000)
            statistics.sample(now + 1_000_000_000)
            assertTrue(statistics.fps > 0f)
            assertEquals(0f, receiver.getFps()); assertEquals(0f, receiver.getMbps())
            repeat(3) {
                receiver.stop()
                assertThrows(IllegalStateException::class.java) { receiver.start() }
                assertNull(job(receiver))
            }
        } finally { receiver.stop(); original?.join() }
    }

    private class BlockingSocket : Socket() {
        val entered = CountDownLatch(1)
        private val released = CountDownLatch(1)
        override fun connect(endpoint: SocketAddress?, timeout: Int) {}
        override fun setTcpNoDelay(value: Boolean) {}
        override fun setSoTimeout(value: Int) {}
        override fun setReceiveBufferSize(value: Int) {}
        override fun getInputStream(): InputStream = object : InputStream() {
            override fun read(): Int { entered.countDown(); released.await(); return -1 }
        }
        override fun close() { super.close(); released.countDown() }
    }

    @Test fun t1181TerminalStopRetiresOriginalSocketCodecAndBorrowsSurfaceUntilCallerRelease() = runBlocking {
        StartupCodecShadow.stage = ""
        StartupCodecShadow.released = CountDownLatch(1)
        val socket = BlockingSocket()
        val surface = Surface(SurfaceTexture(1))
        val receiver = VideoReceiver(initialGeneration = Long.MAX_VALUE - 1) { socket }
        receiver.setNativeSurface(surface)
        var original: Job? = null
        try {
            receiver.start()
            original = checkNotNull(job(receiver))
            assertTrue("T1181 socket not reached: ${org.robolectric.shadows.ShadowLog.getLogsForTag("BlentVideo").joinToString { "${it.msg}: ${it.throwable}" }}", socket.entered.await(3, TimeUnit.SECONDS))
            assertNotNull(receiver.decoder.mediaCodec)
            receiver.stop(); original.join()
            awaitDecoderRetirement(receiver.decoder)
            assertTrue(socket.isClosed)
            assertEquals(0L, StartupCodecShadow.released.count)
            assertNull(receiver.decoder.mediaCodec)
            assertTrue(surface.isValid)
            assertEquals(Long.MAX_VALUE, generation(receiver).get())
            assertFalse(current(receiver, Long.MAX_VALUE))
            assertThrows(IllegalStateException::class.java) { receiver.start() }
            assertNull(job(receiver))
        } finally {
            receiver.stop(); socket.close(); original?.join()
            awaitDecoderRetirement(receiver.decoder); surface.release()
        }
    }
    @Test fun t1181NegativeSeedsRejectBeforeAnyJobOrNativeEffects() {
        var opened = 0
        for (seed in listOf(Long.MIN_VALUE, -1L)) {
            assertThrows(IllegalArgumentException::class.java) {
                VideoReceiver(initialGeneration = seed) { opened++; error("Unexpected socket") }
            }
        }
        assertEquals(0, opened)
    }

    @Test fun t1181BoundedWideModelPreservesEveryStartStopAndTerminalDenial() = runBlocking {
        for (seed in (0L..16L) + (Long.MAX_VALUE - 16..Long.MAX_VALUE)) checkSeed(seed)
    }

    private suspend fun checkSeed(seed: Long) {
        val receiver = VideoReceiver(initialGeneration = seed) { error("No Surface") }
        val maximum = BigInteger.valueOf(Long.MAX_VALUE)
        var expected = BigInteger.valueOf(seed)
        try {
            repeat(20) {
                val result = runCatching { receiver.start() }
                val original = job(receiver)
                if (expected < maximum) {
                    expected += BigInteger.ONE
                    result.getOrThrow()
                    assertNotNull(original)
                } else {
                    assertTrue(result.exceptionOrNull() is IllegalStateException)
                    assertNull(original)
                }
                assertEquals(expected, BigInteger.valueOf(generation(receiver).get()))
                receiver.stop(); original?.join()
                assertEquals(expected, BigInteger.valueOf(generation(receiver).get()))
                assertFalse(current(receiver, expected.toLong()))
            }
        } finally { val original = job(receiver); receiver.stop(); original?.join() }
    }

    private fun joinedCalls(action: () -> Unit): List<Throwable?> {
        val gate = CountDownLatch(1)
        val outcomes = CopyOnWriteArrayList<Throwable?>()
        val threads = ArrayList<Thread>(8)
        try {
            repeat(8) {
                threads += Thread { gate.await(); outcomes.add(runCatching(action).exceptionOrNull()) }
                threads.last().start()
            }
        } finally { gate.countDown(); threads.forEach { it.join() } }
        return outcomes
    }

    @Test fun t1181ConcurrentTerminalStartStopKeepsOneOriginalJobAndNoRevival() = runBlocking {
        val receiver = VideoReceiver(initialGeneration = Long.MAX_VALUE - 1) { error("No Surface") }
        var original: Job? = null
        try {
            val admitted = joinedCalls { receiver.start() }
            assertEquals(8, admitted.size)
            assertTrue(admitted.all { it == null })
            original = checkNotNull(job(receiver))
            assertEquals(Long.MAX_VALUE, generation(receiver).get())
            receiver.stop(); original.join()
            val rejected = joinedCalls { receiver.start() }
            assertEquals(8, rejected.size)
            assertTrue(rejected.all { it is IllegalStateException })
            assertNull(job(receiver))
            assertFalse(current(receiver, Long.MAX_VALUE))
        } finally { receiver.stop(); original?.join() }
    }

    @Test fun t1181TerminalStopKeepsOriginalBlockedCodecCreatorUntilActualReturn() = runBlocking {
        StartupCodecShadow.stage = ""
        StartupCodecShadow.released = CountDownLatch(1)
        val entered = CountDownLatch(1)
        val release = CountDownLatch(1)
        val surface = Surface(SurfaceTexture(1))
        val receiver = VideoReceiver(initialGeneration = Long.MAX_VALUE - 1) { error("Retired creator cannot connect") }
        receiver.decoder.createCodec = { format ->
            entered.countDown(); release.await(); MediaCodec.createDecoderByType(format.mimeType)
        }
        receiver.setNativeSurface(surface)
        var original: Job? = null
        try {
            receiver.start(); original = checkNotNull(job(receiver))
            assertTrue(entered.await(3, TimeUnit.SECONDS))
            receiver.stop()
            assertFalse(original.isCompleted)
            assertFalse(current(receiver, Long.MAX_VALUE))
            assertThrows(IllegalStateException::class.java) { receiver.start() }
            assertNull(job(receiver))
            release.countDown(); original.join()
            awaitDecoderRetirement(receiver.decoder)
            assertTrue(original.isCompleted)
            assertEquals(0L, StartupCodecShadow.released.count)
            assertNull(receiver.decoder.mediaCodec)
            assertTrue(surface.isValid)
        } finally {
            release.countDown(); receiver.stop(); original?.join()
            awaitDecoderRetirement(receiver.decoder); surface.release()
        }
    }

}
