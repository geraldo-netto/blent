package com.blent

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.os.Looper
import kotlinx.coroutines.runBlocking
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.RuntimeEnvironment
import org.robolectric.Shadows.shadowOf
import org.robolectric.annotation.Config
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicInteger

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [27, 34])
class CodecVersionTest {
    @org.junit.Before fun workerBridge() { WorkerTestBridge.install() }
    @Test fun t1206OldShellFormatterIsRejected() {
        assertThrows(IllegalArgumentException::class.java) { decoderReport(1) { emptyArray() } }
        assertThrows(IllegalArgumentException::class.java) { formatDecoderReport(1, null) }
    }

    @Test fun t1206WorkerBoundaryRejectsOldAndInvalidShellTags() {
        for (version in -8..8) {
            val request = MediaTask.Metadata(640, 480, 30, shellVersion = version)
            val accepted = runCatching { MediaTaskJson.decode(MediaTaskJson.encode(request)) }.isSuccess
            assertEquals("T1206 shell tag $version", version == 0 || version == 2, accepted)
        }
        for (version in listOf(Int.MIN_VALUE, Int.MAX_VALUE)) {
            val request = MediaTask.Metadata(shellVersion = version)
            assertTrue(runCatching { MediaTaskJson.decode(MediaTaskJson.encode(request)) }.isFailure)
        }
    }

    @Test fun t1206InvalidRemoteShellTagRejectsBeforeWorkerAdmission() = runBlocking {
        val context = RuntimeEnvironment.getApplication()
        for (version in listOf(-1, 0, 1, 3, Int.MIN_VALUE, Int.MAX_VALUE)) {
            assertTrue("T1206 remote shell tag $version", runCatching { RemoteMetadata.shell(context, version) }.exceptionOrNull() is IllegalArgumentException)
        }
    }

    @Test fun t1206OrderedBroadcastRejectsOldMissingAndInvalidVersionsBeforeQuery() {
        val context = RuntimeEnvironment.getApplication()
        val queries = AtomicInteger()
        val receiver = CodecReportReceiver { queries.incrementAndGet(); "called" }
        val action = "com.blent.T1206"
        context.registerReceiver(receiver, IntentFilter(action), Context.RECEIVER_EXPORTED)
        try {
            for (version in listOf(null, -1, 0, 1, 3, Int.MAX_VALUE)) {
                val intent = Intent(action)
                if (version != null) intent.putExtra("blent_codecs_version", version)
                val result = receive(context, intent)
                assertEquals("T1206 invalid broadcast result $version", "unprocessed", result)
                assertEquals("T1206 query admitted for $version", 0, queries.get())
            }
        } finally { context.unregisterReceiver(receiver) }
    }

    private fun receive(context: Context, intent: Intent): String? {
        var complete = false
        var result: String? = null
        val observer = object : BroadcastReceiver() {
            override fun onReceive(context: Context, intent: Intent) { result = resultData; complete = true }
        }
        context.sendOrderedBroadcast(intent, null, observer, null, 0, "unprocessed", null)
        val deadline = System.nanoTime() + TimeUnit.SECONDS.toNanos(3)
        while (!complete && System.nanoTime() < deadline) {
            shadowOf(Looper.getMainLooper()).idle()
            Thread.sleep(1)
        }
        assertTrue("T1206 ordered broadcast did not complete", complete)
        return result
    }
}
