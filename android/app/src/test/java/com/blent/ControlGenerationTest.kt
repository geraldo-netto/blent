package com.blent

import okhttp3.*
import okio.ByteString
import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [27, 34])
class ControlGenerationTest {
    private class Input : ControlInputState {
        override fun reset() {}
        override fun forgetTouches() {}
        override fun setTouchEnabled(enabled: Boolean) {}
        override fun setPenEnabled(enabled: Boolean) {}
    }
    private class Socket(val listener: WebSocketListener) : WebSocket {
        val sent = mutableListOf<String>()
        var rejected = false
        var stopped = false
        override fun request() = Request.Builder().url("ws://localhost/").build()
        override fun queueSize() = 0L
        override fun send(text: String): Boolean { if (rejected) return false; sent.add(text); return true }
        override fun send(bytes: ByteString) = false
        override fun close(code: Int, reason: String?): Boolean { stopped = true; return true }
        override fun cancel() { stopped = true }
        fun open() = listener.onOpen(this, Response.Builder().request(request()).protocol(Protocol.HTTP_1_1).code(101).message("test").build())
        fun configurations() = sent.count { JSONObject(it).optString("type") == "config" }
    }
    private class Rig(connection: Long = 0, settings: Long = 0) : AutoCloseable {
        val sockets = mutableListOf<Socket>()
        val control = ControlSession(Any(), Input(), WebSocket.Factory { _, listener -> Socket(listener).also { sockets.add(it) } },
            initialConnectionGeneration = connection, initialSettingsGeneration = settings)
        fun open() { control.connect(); sockets.last().open() }
        override fun close() { control.disconnect() }
    }

    @Test fun t1179TerminalConnectionNeverCreatesANativeSocket() {
        Rig(connection = Long.MAX_VALUE).use { rig ->
            repeat(4) { rig.control.connect(); rig.control.disconnect() }
            assertTrue("T1179 exhausted connection created native sockets", rig.sockets.isEmpty())
        }
    }

    @Test fun t1179LastConnectionCannotReconnectAfterOriginalSendRefusal() {
        Rig(connection = Long.MAX_VALUE - 1).use { rig ->
            rig.open()
            val original = rig.sockets.single()
            original.rejected = true
            rig.control.sendWhenConnected(JSONObject().put("type", "test"))
            assertTrue(original.stopped)
            rig.control.connect()
            assertEquals("T1179 refusal wrapped into another socket admission", 1, rig.sockets.size)
        }
    }

    @Test fun t1179TerminalSettingsNeverEnterTheNativeQueue() {
        Rig(settings = Long.MAX_VALUE).use { rig ->
            rig.open()
            repeat(4) { rig.control.sendConfig(20000, 60) }
            assertEquals("T1179 settings counter wrapped", 0, rig.sockets.single().configurations())
        }
    }

    @Test fun t1179DeniedSettingsCannotReplayTheLastPendingRequestOnReconnect() {
        Rig(settings = Long.MAX_VALUE - 1).use { rig ->
            rig.open()
            rig.control.sendConfig(20000, 60)
            assertEquals(1, rig.sockets.single().configurations())
            rig.control.sendConfig(12000, 30)
            assertEquals("T1179 second request passed exhaustion", 1, rig.sockets.single().configurations())
            rig.control.disconnect(); rig.open()
            assertEquals("T1179 denied pending settings replayed", 0, rig.sockets.last().configurations())
        }
    }
    @Test fun t1179EpochModelRetiresEveryOldNoticeWithoutResettingCapacity() {
        val limit = java.math.BigInteger.valueOf(Long.MAX_VALUE)
        for (seed in listOf(0, Long.MAX_VALUE - 3, Long.MAX_VALUE - 2, Long.MAX_VALUE - 1, Long.MAX_VALUE)) {
            val epoch = if (seed == 0L) ControlEpoch() else ControlEpoch(seed)
            val foreign = ControlEpoch(seed)
            var model = java.math.BigInteger.valueOf(seed)
            val old = mutableListOf<ControlNotice>()
            repeat(32) { step ->
                old.add(epoch.snapshot())
                if (step % 3 == 0) epoch.revoke() else {
                    val permitted = model < limit
                    assertEquals(permitted, epoch.acquire())
                    if (permitted) model += java.math.BigInteger.ONE
                }
                assertEquals(model < limit, epoch.available())
                assertTrue(epoch.current(epoch.snapshot()))
                assertFalse(foreign.current(epoch.snapshot()))
                assertTrue(old.none { epoch.current(it) })
            }
        }
    }

    @Test fun t1179InvalidInitialEpochIsRejected() {
        assertThrows(IllegalArgumentException::class.java) { ControlEpoch(-1) }
        assertThrows(IllegalArgumentException::class.java) { ControlEpoch(Long.MIN_VALUE) }
    }

    @Test fun t1179QueuedModeCannotSurviveTerminalConnectionCutoff() {
        val sockets = mutableListOf<Socket>()
        val capture = TouchCapture(WebSocket.Factory { _, listener -> Socket(listener).also { sockets.add(it) } }, initialConnectionGeneration = Long.MAX_VALUE - 1)
        val prefs = Prefs(org.robolectric.RuntimeEnvironment.getApplication()).apply { hostToken = "ab".repeat(32); checkUpdates = false; hasUserSettings = false }
        val callbacks = mutableListOf<() -> Unit>()
        val receiver = VideoReceiver { error("T1179 no native video") }
        val session = SessionCoordinator(prefs, { callbacks.add(it) }, receiver, capture)
        try {
            session.start(); sockets.single().open()
            receiver.start()
            val old = org.robolectric.util.ReflectionHelpers.getField<kotlinx.coroutines.Job>(receiver, "job")
            capture.onModeKnown!!.invoke(true)
            val notice = capture.connectionSnapshot
            capture.disconnect(); capture.connect()
            callbacks.toList().forEach { it() }
            assertFalse(capture.isCurrentConnection(notice))
            assertFalse(session.penOnlyMode)
            assertTrue("T1179 terminal close notification did not retire video", old.isCancelled)
            assertEquals(1, sockets.size)
        } finally { session.stop() }
    }

    @Test fun t1179QueuedRejectionCannotRestoreDeniedSettings() {
        val sockets = mutableListOf<Socket>()
        val capture = TouchCapture(WebSocket.Factory { _, listener -> Socket(listener).also { sockets.add(it) } }, initialSettingsGeneration = Long.MAX_VALUE - 1)
        val prefs = Prefs(org.robolectric.RuntimeEnvironment.getApplication()).apply { hostToken = "ab".repeat(32); checkUpdates = false; hasUserSettings = false }
        val callbacks = mutableListOf<() -> Unit>()
        val session = SessionCoordinator(prefs, { callbacks.add(it) }, null, capture)
        try {
            session.start(); sockets.single().open()
            session.handle(SettingsEvent.Stream(20000, 60))
            val notice = capture.settingsSnapshot
            capture.onSettingsRejected!!.invoke(RejectedStreamSettings("old", 9000, 30, notice))
            session.handle(SettingsEvent.Stream(12000, 60))
            callbacks.toList().forEach { it() }
            assertFalse(capture.isCurrentSettings(notice))
            assertEquals(12000, prefs.bitrateKbps)
            assertEquals(60, prefs.fps)
            assertEquals(1, sockets.single().configurations())
        } finally { session.stop() }
    }

    @Test fun t1179NativeRejectionCarriesCurrentNoticeAndConfirmedReplay() {
        Rig().use { rig ->
            rig.open()
            rig.control.sendConfig(20000, 60)
            val notices = mutableListOf<RejectedStreamSettings>()
            rig.control.onSettingsRejected = { notices.add(it) }
            val socket = rig.sockets.single()
            fun reject(fps: Int, bitrate: Int, requested: Int = 20000) {
                socket.listener.onMessage(socket, JSONObject().put("status", "settings_rejected")
                    .put("fps", fps).put("bitrate", bitrate)
                    .put("requested", JSONObject().put("bitrate", requested).put("fps", 60)).toString())
            }
            reject(30, 9000, 12000)
            reject(9, 9000)
            reject(30, 999)
            assertTrue(notices.isEmpty())
            reject(30, 9000)
            assertEquals(1, notices.size)
            assertTrue(rig.control.isCurrentSettings(notices.single().requestSnapshot))
            rig.control.disconnect(); rig.open()
            val replay = JSONObject(rig.sockets.last().sent.single { JSONObject(it).optString("type") == "config" })
            assertEquals(30, replay.getInt("fps"))
            assertEquals(9000, replay.getInt("bitrate"))
        }
    }

    @Test fun t1179StaleReceiptCannotQueueAtTerminalConnection() {
        Rig(connection = Long.MAX_VALUE - 1).use { rig ->
            rig.control.sendRendered(0, 0)
            rig.open()
            val original = rig.control.connectionSnapshot
            rig.control.sendRendered(RenderAcknowledgement(1, 10, null) { rig.control.isCurrentConnection(original) })
            val socket = rig.sockets.single()
            assertEquals(1, socket.sent.count { JSONObject(it).optString("type") == "rendered" })
            rig.control.disconnect(); rig.control.connect()
            rig.control.sendRendered(RenderAcknowledgement(2, 10, null) { rig.control.isCurrentConnection(original) })
            assertEquals(1, socket.sent.count { JSONObject(it).optString("type") == "rendered" })
            assertEquals(1, rig.sockets.size)
        }
    }

}
