package com.blent

import android.graphics.SurfaceTexture
import android.view.Surface
import android.view.SurfaceHolder
import android.view.SurfaceView
import kotlinx.coroutines.*
import okhttp3.*
import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.RuntimeEnvironment
import org.robolectric.annotation.Config
import java.lang.reflect.Proxy

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [27, 34])
class DisplayActivityLifecycleTest {
    private class Rig(hold: Boolean = false, rejectInitially: Boolean = false) {
        val scope = CoroutineScope(SupervisorJob() + Dispatchers.Unconfined)
        val retired = CompletableDeferred<Unit>().apply { if (!hold) complete(Unit) }
        val requests = ArrayList<MediaTask.Display>()
        val events = ArrayList<(WorkerEvent) -> Unit>()
        var settled = 0
        var cancelled = 0
        val surface = Surface(SurfaceTexture(0))
        val receiver = RemoteDisplay(RuntimeEnvironment.getApplication(), scope) { task, _, report ->
            requests.add(task); events.add(report)
            object : LifecycleRun {
                override suspend fun execute() = awaitCancellation()
                override fun cancel() { cancelled++ }
                override suspend fun retire() { retired.await(); settled++ }
            }
        }
        val sockets = ArrayList<Pair<DisplayRouteSocket, WebSocketListener>>()
        val control = TouchCapture(WebSocket.Factory { request, listener ->
            DisplayRouteSocket(request).also { sockets.add(it to listener) }
        })
        val actions = java.util.concurrent.ConcurrentLinkedQueue<() -> Unit>()
        val prefs = Prefs(RuntimeEnvironment.getApplication()).apply { hostToken = "ab".repeat(32); checkUpdates = false }
        val rejectUi = java.util.concurrent.atomic.AtomicBoolean(rejectInitially)
        private val uiThread = Thread.currentThread()
        val session = SessionCoordinator(prefs, { action ->
            if (rejectUi.get() && Thread.currentThread() !== uiThread) throw java.util.concurrent.RejectedExecutionException("T1057 activity UI rejected")
            actions.add(action)
        }, receiver, control)
        init {
            val view = object : SurfaceView(RuntimeEnvironment.getApplication()) {
                override fun getHolder(): SurfaceHolder = Proxy.newProxyInstance(SurfaceHolder::class.java.classLoader,
                    arrayOf(SurfaceHolder::class.java)) { _, method, _ -> if (method.name == "getSurface") surface else null } as SurfaceHolder
            }
            session.surfaceReady(view); session.start()
            val (socket, listener) = sockets.last()
            listener.onOpen(socket, Response.Builder().request(socket.request()).protocol(Protocol.HTTP_1_1).code(101).message("fixture").build())
        }
        fun signal(revision: Long, stopped: Long, active: Boolean, greeting: Boolean = false) {
            val message = JSONObject().put("status", if (greeting) "connected" else "display_activity")
                .put("display_activity", JSONObject().put("version", 1).put("revision", revision).put("stopped", stopped).put("active", active))
            if (greeting) message.put("codec", "h264").put("fps", 30).put("video_width", 64).put("video_height", 32).put("pen_only", false)
            val (socket, listener) = sockets.last()
            listener.onMessage(socket, message.toString())
        }
        fun drain() { while (true) { val action = actions.poll() ?: return; action() } }
        suspend fun until(predicate: () -> Boolean) {
            withTimeout(2000) { while (!predicate()) { drain(); delay(1) } }
            drain()
        }
        suspend fun awaitRequests(count: Int) {
            withTimeout(2000) { while (requests.size != count) { drain(); delay(1) } }
            drain()
        }
        suspend fun close() {
            retired.complete(Unit); session.stop(); drain()
            withTimeout(2000) { while (settled != requests.size) yield() }
            scope.cancel(); surface.release()
        }
    }
    @Test fun t1057InactiveHostGreetingCannotCreateDecoderAndResumeKeepsConfiguration() = runBlocking {
        val rig = Rig()
        try {
            rig.signal(1, 0, false, true); rig.drain()
            assertTrue("T1057 inactive host admitted a decoder", rig.requests.isEmpty())
            rig.signal(2, 0, true); rig.awaitRequests(1)
            assertEquals(1, rig.requests.size)
            assertEquals(64, rig.requests.single().format.width)
            assertEquals(30, rig.requests.single().format.fps)
        } finally { rig.close() }
    }
    @Test fun t1057UnsupportedFormatCannotBeOverriddenByRecognizedMimeChange() = runBlocking {
        val rig = Rig()
        try {
            rig.signal(1, 0, true, true); rig.awaitRequests(1)
            assertEquals(1, rig.requests.size)
            rig.control.onStreamFormat!!.invoke(null)
            rig.control.onCodecKnown!!.invoke("hevc")
            rig.drain()
            assertEquals("T1057 MIME change overrode unsupported decoder format", 1, rig.requests.size)
        } finally { rig.close() }
    }
    @Test fun t1057CoalescedResumeCannotBypassOriginalHeldRetirementOrReviveLateFrames() = runBlocking {
        val rig = Rig(hold = true)
        try {
            rig.signal(1, 0, true, true); rig.awaitRequests(1)
            val old = rig.events.single()
            val receipts = ArrayList<RenderAcknowledgement>()
            rig.receiver.onFrameRendered = { receipts.add(it) }
            old(WorkerEvent.Rendered(7, 2, null))
            assertTrue(receipts.single().current())
            rig.signal(2, 1, true)
            rig.until { rig.cancelled > 0 }
            assertEquals(0, rig.settled)
            assertFalse(receipts.single().current())
            repeat(32) { rig.signal(3L + it, 2L + it, true) }
            assertTrue("T1057 unbounded UI hint queue", rig.actions.size <= 1)
            rig.drain()
            old(WorkerEvent.Rendered(8, 2, null))
            assertEquals(1, receipts.size)
            assertEquals("T1057 held original decoder was reused", 1, rig.requests.size)
            rig.retired.complete(Unit)
            rig.awaitRequests(2)
            assertEquals(1, rig.settled)
            assertEquals(rig.requests.first().routes, rig.requests.last().routes)
            assertEquals(rig.requests.first().format, rig.requests.last().format)
        } finally { rig.close() }
    }
    @Test fun t1057QueuedActiveHintCannotReopenAfterBackgroundOrMalformedCurrentSignal() = runBlocking {
        val rig = Rig()
        try {
            rig.signal(1, 0, true, true)
            rig.session.stop(); rig.drain()
            assertTrue(rig.requests.isEmpty())
            rig.session.start()
            val (socket, listener) = rig.sockets.last()
            listener.onOpen(socket, Response.Builder().request(socket.request()).protocol(Protocol.HTTP_1_1).code(101).message("new owner").build())
            rig.signal(1, 0, true, true); rig.awaitRequests(1)
            val count = rig.requests.size
            listener.onMessage(socket, """{"status":"display_activity","display_activity":{"version":1,"revision":2,"stopped":0,"active":false}}""")
            rig.until { rig.settled == 1 }
            assertTrue(socket.cancelled)
            rig.signal(3, 1, true)
            rig.drain()
            assertEquals(count, rig.requests.size)
        } finally { rig.close() }
    }

    @Test fun t1057PausedSourceCannotReopenThroughConfigurationCallbacksAndLegacyStillStarts() = runBlocking {
        val rig = Rig()
        try {
            val (socket, listener) = rig.sockets.last()
            listener.onMessage(socket, """{"status":"connected","codec":"h264","fps":30,"video_width":64,"video_height":32,"pen_only":false}""")
            rig.awaitRequests(1)
            // A new original control scope may announce activity support.
            listener.onFailure(socket, java.io.IOException("replace legacy scope"), null)
            rig.session.start()
            val (next, current) = rig.sockets.last()
            current.onOpen(next, Response.Builder().request(next.request()).protocol(Protocol.HTTP_1_1).code(101).message("supported").build())
            rig.signal(1, 0, true, true); rig.awaitRequests(2)
            rig.signal(2, 1, false)
            rig.control.onFpsKnown!!.invoke(60)
            rig.control.onVideoLivenessKnown!!.invoke(true)
            rig.session.handle(SettingsEvent.Stream(15000, 30))
            rig.until { rig.settled == 2 }
            assertEquals(2, rig.requests.size)
            assertEquals(15000, rig.prefs.bitrateKbps)
            rig.signal(3, 1, true); rig.awaitRequests(3)
        } finally { rig.close() }
    }

    @Test fun t1057RejectedSettingsCannotRestartBeforeQueuedPauseIsApplied() = runBlocking {
        val rig = Rig()
        try {
            rig.signal(1, 0, true, true); rig.awaitRequests(1)
            rig.signal(2, 1, false)
            rig.control.onSettingsRejected!!.invoke(RejectedStreamSettings("rejected", 12000, 60, rig.control.settingsSnapshot))
            // Execute the settings callback first, independent of UI hint scheduling.
            val pending = ArrayList<() -> Unit>()
            while (true) { pending.add(rig.actions.poll() ?: break) }
            pending.asReversed().forEach { it() }
            rig.until { rig.settled >= 1 }
            assertEquals("T1057 rejected settings reopened a paused display", 1, rig.requests.size)
            assertEquals(60, rig.prefs.fps)
        } finally { rig.close() }
    }

    @Test fun t1057ThrowingOptionalObserverCannotSkipOriginalControlRevocation() = runBlocking {
        val rig = Rig()
        val observer = rig.control.onDisplayActivityKnown
        try {
            rig.control.onDisplayActivityKnown = { throw IllegalStateException("T1057 optional observer") }
            rig.signal(1, 0, false, true)
            assertTrue(rig.control.connectionState.value.authenticated)
            rig.control.disconnect()
            assertFalse(rig.control.connectionState.value.authenticated)
            assertSame(com.blent.activity.DisplayActivityState.Unavailable, rig.control.displayActivityUpdate.state)
            rig.drain(); assertTrue(rig.requests.isEmpty())
        } finally { rig.control.onDisplayActivityKnown = observer; rig.close() }
    }
    @Test fun t1057RejectedUiDeliveryRetainsNoDecoderOrOffThreadActivityMutation() = runBlocking {
        val rig = Rig(rejectInitially = true)
        try {
            rig.rejectUi.set(true)
            rig.signal(1, 0, true, true)
            rig.until { rig.session.settings.streamError != null || org.robolectric.shadows.ShadowLog.getLogsForTag("Blent").any { it.msg == "Display activity UI unavailable" } }
            assertNull("T1057 rejected dispatcher wrote Activity state off its owner thread", rig.session.settings.streamError)
            assertTrue(rig.requests.isEmpty())
            rig.rejectUi.set(false)
            rig.signal(2, 0, true); rig.awaitRequests(1)
        } finally { rig.close() }
    }

}
