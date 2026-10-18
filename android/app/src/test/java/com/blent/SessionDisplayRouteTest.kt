package com.blent

import android.os.Looper
import kotlinx.coroutines.*
import okhttp3.*
import okio.ByteString
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.RuntimeEnvironment
import org.robolectric.Shadows.shadowOf
import org.robolectric.annotation.Config
import org.robolectric.annotation.LooperMode
import org.robolectric.util.ReflectionHelpers

internal class DisplayRouteSocket(private val request: Request) : WebSocket {
    var cancelled = false
    val messages = ArrayList<String>()
    override fun request() = request
    override fun queueSize() = 0L
    override fun send(text: String): Boolean { messages.add(text); return !cancelled }
    override fun send(bytes: ByteString) = !cancelled
    override fun close(code: Int, reason: String?): Boolean { cancelled = true; return true }
    override fun cancel() { cancelled = true }
}
internal class SessionDisplayRig(sequence: DisplayRouteSequence = DisplayRouteSequence()) {
    val prefs = Prefs(RuntimeEnvironment.getApplication()).apply { checkUpdates = false; hostToken = "a".repeat(64) }
    val sockets = ArrayList<Pair<DisplayRouteSocket, WebSocketListener>>()
    val control = TouchCapture(WebSocket.Factory { request, listener ->
        DisplayRouteSocket(request).also { sockets.add(it to listener) }
    })
    val receiver = VideoReceiver { error("T907 no native video without a Surface") }
    val callbacks = java.util.concurrent.ConcurrentLinkedQueue<() -> Unit>()
    val session = SessionCoordinator(prefs, { callbacks.add(it) }, receiver, control, routeSequence = sequence)
    val endpoint = DisplayEndpoint("b".repeat(64), 23456, 23457)
    fun job(): Job? = ReflectionHelpers.getField(receiver, "job")
    fun drain() { while (true) { val action = callbacks.poll() ?: return; action() } }
    suspend fun greet() {
        val (socket, listener) = sockets.last()
        listener.onOpen(socket, Response.Builder().request(socket.request()).protocol(Protocol.HTTP_1_1).code(101).message("T907 current source").build())
        listener.onMessage(socket, """{"status":"connected","codec":"h264","fps":30,"video_width":64,"video_height":32,"pen_only":false}""")
        withTimeout(2000) { while (job() == null) { drain(); delay(1) } }
        drain()
    }
}

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [27, 34])
@LooperMode(LooperMode.Mode.PAUSED)
class SessionDisplayRouteTest {
    @Test fun t907RouteSelectionRetiresOldControlAndVideoBeforeApplyingNewCredentialAndPorts() = runBlocking {
        val rig = SessionDisplayRig()
        with(rig) {
            session.start(); receiver.start(); val oldJob = checkNotNull(job())
            control.onModeKnown!!.invoke(false)
            val old = sockets.single().first
            try {
                session.selectDisplay(endpoint)
                assertTrue(old.cancelled); assertTrue(oldJob.isCancelled)
                assertEquals(endpoint.token, control.token); assertEquals(endpoint.token, receiver.token)
                assertEquals(endpoint.controlPort, sockets.last().first.request().url.port)
                assertEquals(endpoint.videoPort, receiver.routes?.videoPort)
                assertEquals(control.routes, receiver.routes); assertEquals(2L, receiver.routes?.revision)
                drain(); assertNull(job())
                greet(); assertNotNull(job())
                session.selectDisplay(null); drain()
                assertNull(control.routes); assertNull(receiver.routes); assertNull(job())
                assertTrue(sockets.last().first.cancelled)
            } finally { session.stop() }
        }
    }
    @Test fun t907ForegroundLifecycleCannotRestoreRevokedRoutesFromPersistedAdbCredentials() {
        val rig = SessionDisplayRig()
        with(rig) {
            try {
                session.selectDisplay(endpoint); assertTrue(sockets.isEmpty()); assertNull(job())
                session.start(); assertEquals(1, sockets.size)
                session.stop(); session.selectDisplay(null); session.start()
                assertEquals(1, sockets.size); assertNull(control.routes); assertNull(receiver.routes)
                prefs.hostToken = "c".repeat(64); shadowOf(Looper.getMainLooper()).idle()
                assertEquals(2, sockets.size); assertEquals(AdbDisplayRoutes.create().controlPort, sockets.last().first.request().url.port)
                assertEquals(3L, control.routes?.revision)
                session.selectDisplay(endpoint); assertEquals(4L, control.routes?.revision)
                prefs.hostToken = "d".repeat(64); shadowOf(Looper.getMainLooper()).idle()
                assertEquals(endpoint.token, control.token); assertEquals(3, sockets.size)
                prefs.hostToken = null; shadowOf(Looper.getMainLooper()).idle()
                assertEquals(endpoint.token, receiver.token)
            } finally { session.stop() }
        }
    }
    @Test fun t907BackgroundReplacementStaysStoppedUntilForegroundAndUsesNewestPorts() {
        val rig = SessionDisplayRig()
        with(rig) {
            try {
                session.start(); session.stop(); session.selectDisplay(endpoint)
                session.selectDisplay(endpoint.copy(controlPort = 34567))
                assertEquals(1, sockets.size); assertNull(job()); assertEquals(34567, control.routes?.controlPort)
                session.start(); assertEquals(2, sockets.size); assertEquals(34567, sockets.last().first.request().url.port)
            } finally { session.stop() }
        }
    }
    @Test fun t907RevisionExhaustionClearsOldAuthorityAndReportsFailureWithoutCrashing() {
        val rig = SessionDisplayRig(DisplayRouteSequence(Long.MAX_VALUE))
        with(rig) {
            session.start()
            try {
                session.selectDisplay(endpoint)
                assertNull(control.routes); assertNull(receiver.routes); assertTrue(sockets.single().first.cancelled)
                assertEquals("Display route revisions exhausted", session.settings.streamError)
                prefs.hostToken = "c".repeat(64); session.applyToken(true)
                assertNull(control.routes); assertEquals(1, sockets.size)
            } finally { session.stop() }
        }
    }
    @Test fun t907PartialRouteInstallationFailureRetiresBothChannelsAndKeepsErrorVisible() {
        val rig = SessionDisplayRig()
        val broken = object : DisplaySession<android.view.SurfaceView> by rig.receiver {
            override var routes: DisplayRoutes?
                get() = rig.receiver.routes
                set(value) { rig.receiver.routes = value; if (value != null) error("endpoint rejected") }
        }
        val session = SessionCoordinator(rig.prefs, {}, broken, rig.control)
        session.start()
        try {
            session.selectDisplay(rig.endpoint)
            assertNull(rig.control.routes); assertNull(rig.receiver.routes)
            assertEquals("endpoint rejected", session.settings.streamError)
            assertTrue(rig.sockets.all { it.first.cancelled })
        } finally { session.stop() }
    }
}
