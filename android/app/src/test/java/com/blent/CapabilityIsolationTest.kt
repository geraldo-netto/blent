package com.blent

import android.media.MediaCodecInfo
import kotlinx.coroutines.*
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config
import java.util.concurrent.*

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [27, 34])
class CapabilityIsolationTest {
    @org.junit.Before fun workerBridge() { WorkerTestBridge.install() }
    @Test fun t808_busyNativeWorkerReturnsUnavailableWithoutNewQueries() = runBlocking {
        val entered = CountDownLatch(1)
        val release = CountDownLatch(1)
        val first = async(Dispatchers.Default) { MetadataQueries.lane.query { entered.countDown(); release.await(); 1 } }
        val context = org.robolectric.RuntimeEnvironment.getApplication()
        var result: String? = null
        val completion = object : android.content.BroadcastReceiver() {
            override fun onReceive(context: android.content.Context, intent: android.content.Intent) { result = resultData }
        }
        try {
            assertTrue(entered.await(2, TimeUnit.SECONDS))
            val report = DecoderCapabilities.report(640, 480, 30, { error("T808 replacement query") }, { _, _ -> error("T808 replacement query") })
            assertEquals("unavailable", report.getString("status"))
            assertEquals(0, report.getJSONArray("codecs").length())
            assertEquals(0, report.getJSONArray("hardware").length())
            assertEquals(0, report.getJSONArray("details").length())
            context.sendOrderedBroadcast(android.content.Intent(context, CodecReportReceiver::class.java)
                .putExtra("blent_codecs_version", 2), null, completion, null, 0, null, null)
            val deadline = System.nanoTime() + TimeUnit.SECONDS.toNanos(3)
            while (result == null && System.nanoTime() < deadline) {
                org.robolectric.Shadows.shadowOf(android.os.Looper.getMainLooper()).idle()
                delay(5)
            }
            assertEquals("BLENT_CODECS_V2:h264=unknown;hevc=unknown;vp9=unknown;av1=unknown", result)
        } finally { release.countDown(); first.await() }
    }

    @Test fun t808_broadcastDispatchDoesNotWaitForNativeQuery() {
        val context = org.robolectric.RuntimeEnvironment.getApplication()
        val entered = CountDownLatch(1)
        val release = CountDownLatch(1)
        val finished = CountDownLatch(1)
        val receiver = CodecReportReceiver { entered.countDown(); release.await(); "test report" }
        val timer = Executors.newSingleThreadScheduledExecutor()
        val completion = object : android.content.BroadcastReceiver() {
            override fun onReceive(context: android.content.Context, intent: android.content.Intent) {
                assertEquals("test report", resultData); finished.countDown()
            }
        }
        context.registerReceiver(receiver, android.content.IntentFilter("com.blent.T808"), android.content.Context.RECEIVER_EXPORTED)
        try {
            timer.schedule({ release.countDown() }, 1, TimeUnit.SECONDS)
            context.sendOrderedBroadcast(android.content.Intent("com.blent.T808").putExtra("blent_codecs_version", 2), null, completion, null, 0, null, null)
            val started = System.nanoTime()
            org.robolectric.Shadows.shadowOf(android.os.Looper.getMainLooper()).idle()
            assertTrue("T808 native query blocked broadcast dispatch", System.nanoTime() - started < TimeUnit.MILLISECONDS.toNanos(300))
            assertTrue(entered.await(2, TimeUnit.SECONDS))
            release.countDown()
            val deadline = System.nanoTime() + TimeUnit.SECONDS.toNanos(3)
            while (finished.count != 0L && System.nanoTime() < deadline) {
                org.robolectric.Shadows.shadowOf(android.os.Looper.getMainLooper()).idle()
                Thread.sleep(5)
            }
            assertEquals(0, finished.count)
        } finally { release.countDown(); timer.shutdownNow(); context.unregisterReceiver(receiver) }
    }

    @Test fun t808_cancellationReturnsWhileNativeQueryStillOwnsWorker() {
        val entered = CountDownLatch(1)
        val release = CountDownLatch(1)
        val executor = Executors.newSingleThreadExecutor()
        val future = executor.submit<Boolean> {
            runBlocking {
                try {
                    withTimeout(100) {
                        DecoderCapabilities.report(640, 480, 30, {
                            entered.countDown(); release.await(); emptyArray<MediaCodecInfo>()
                        }, { _, _ -> error("empty inventory") })
                    }
                    false
                } catch (_: TimeoutCancellationException) { true }
            }
        }
        try {
            assertTrue(entered.await(3, TimeUnit.SECONDS))
            assertTrue("T808 cancellation waits for native query", future.get(1, TimeUnit.SECONDS))
        } finally {
            release.countDown()
            executor.shutdown()
            assertTrue(executor.awaitTermination(3, TimeUnit.SECONDS))
        }
    }
}
