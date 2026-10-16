package com.blent

import android.os.Looper
import kotlinx.coroutines.Job
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

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [27, 34])
@LooperMode(LooperMode.Mode.PAUSED)
class SessionRevocationTest {
    private class Socket : WebSocket {
        var cancelled = false
        override fun request() = Request.Builder().url(AdbDisplayRoutes.controlUrl).build()
        override fun queueSize() = 0L
        override fun send(text: String) = !cancelled
        override fun send(bytes: ByteString) = !cancelled
        override fun close(code: Int, reason: String?): Boolean { cancelled = true; return true }
        override fun cancel() { cancelled = true }
    }
    private class Fixture(token: String? = "a".repeat(64)) {
        val prefs = Prefs(RuntimeEnvironment.getApplication()).apply { checkUpdates = false; hostToken = token }
        val sockets = mutableListOf<Pair<Socket, WebSocketListener>>()
        val capture = TouchCapture(WebSocket.Factory { _, listener -> Socket().also { sockets.add(it to listener) } })
        val receiver = VideoReceiver { error("T904 must never open native video") }
        val callbacks = mutableListOf<() -> Unit>()
        val session = SessionCoordinator(prefs, { callbacks.add(it) }, receiver, capture)
        fun job(): Job? = ReflectionHelpers.getField(receiver, "job")
        fun clear() { prefs.hostToken = null; shadowOf(Looper.getMainLooper()).idle(); session.applyToken(true) }
    }

    @Test fun t904_clearLiveTokenRetiresOldRetriesAndStaleControlCallbacks() {
        val fixture = Fixture()
        with(fixture) {
            session.start()
            receiver.start() // No Surface: job waits, no video or codec I/O.
            val oldJob = checkNotNull(job())
            val oldGeneration = capture.connectionGeneration
            val (socket, listener) = sockets.single()
            capture.onModeKnown!!.invoke(false) // Queue UI callback from the old generation.
            try {
                clear()
                assertTrue("T904 clearing credentials must retire old control", capture.connectionGeneration > oldGeneration)
                assertTrue("T904 clearing credentials must cancel captured video token", oldJob.isCancelled)
                assertTrue(socket.cancelled)
                assertNull(capture.token)
                assertNull(receiver.token)
                callbacks.toList().forEach { it() }
                listener.onFailure(socket, java.io.IOException("late failure"), null)
                assertNull("T904 stale callback must not restart revoked video", job())
                assertEquals(1, sockets.size)
                session.applyToken(true)
                assertEquals(1, sockets.size)
                prefs.hostToken = "b".repeat(64)
                shadowOf(Looper.getMainLooper()).idle()
                assertEquals(2, sockets.size)
                assertEquals(prefs.hostToken, capture.token)
                assertEquals(prefs.hostToken, receiver.token)
                assertNotSame(oldJob, job())
            } finally { session.stop() }
        }
    }

    @Test fun t904_clearWhileBackgroundedDropsCredentialsWithoutStartingWork() {
        val fixture = Fixture()
        with(fixture) {
            session.start(); session.stop()
            try {
                clear()
                assertNull(capture.token)
                assertNull(receiver.token)
                assertNull(job())
                assertEquals(1, sockets.size)
                session.start()
                assertNull(capture.token)
                assertNull(receiver.token)
                assertNull(job())
            } finally { session.stop() }
        }
    }

    @Test fun t904_defaultCompositionDoesNotConnectOrApplyUnconfirmedMode() {
        val prefs = Prefs(RuntimeEnvironment.getApplication()).apply { checkUpdates = false; hostToken = null }
        val session = SessionCoordinator(prefs, { it() })
        try {
            session.handle(SettingsEvent.Mode(true))
            assertFalse(session.penOnlyMode)
            assertFalse(session.presentation.controlConnected.value)
            assertFalse(session.presentation.connected)
            session.applyToken(true)
            assertFalse(session.presentation.controlConnected.value)
        } finally { session.stop() }
    }

    @Test fun t904_initialMissingTokenPreservesExplicitAdbStartupHandshake() {
        val fixture = Fixture(null)
        with(fixture) {
            try {
                session.start()
                val generation = capture.connectionGeneration
                session.applyToken(true)
                assertEquals(generation, capture.connectionGeneration)
                assertEquals(1, sockets.size)
                assertNull(receiver.token)
            } finally { session.stop() }
        }
    }
    @Test fun t1039_queuedOldLivenessOfferCannotRetargetCurrentDisplayRun() {
        val fixture = Fixture()
        with(fixture) {
            session.start()
            receiver.start()
            try {
                capture.onVideoLivenessKnown!!.invoke(true)
                callbacks.toList().forEach { it() }; callbacks.clear()
                assertTrue(ReflectionHelpers.getField(receiver, "livenessEnabled"))
                capture.onVideoLivenessKnown!!.invoke(false)
                callbacks.toList().forEach { it() }; callbacks.clear()
                assertFalse(ReflectionHelpers.getField(receiver, "livenessEnabled"))
                capture.onVideoLivenessKnown!!.invoke(true)
                clear()
                callbacks.toList().forEach { it() }
                assertFalse(ReflectionHelpers.getField(receiver, "livenessEnabled"))
                assertNull(job())
            } finally { session.stop() }
        }
    }

}
