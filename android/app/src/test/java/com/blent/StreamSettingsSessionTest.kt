package com.blent

import okhttp3.*
import okio.ByteString
import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.RuntimeEnvironment
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [27, 34])
class StreamSettingsSessionTest {
    private class Socket(val listener: WebSocketListener) : WebSocket {
        val sent = mutableListOf<String>()
        override fun request() = Request.Builder().url("ws://localhost/").build()
        override fun queueSize() = 0L
        override fun send(text: String): Boolean { sent.add(text); return true }
        override fun send(bytes: ByteString) = false
        override fun close(code: Int, reason: String?) = true
        override fun cancel() {}
        fun open() = listener.onOpen(this, Response.Builder().request(request()).protocol(Protocol.HTTP_1_1).code(101).message("test").build())
        fun message(value: JSONObject) = listener.onMessage(this, value.toString())
        fun greet(bitrate: Int, fps: Int) = message(JSONObject().put("bitrate", bitrate).put("fps", fps))
        fun configs() = sent.map(::JSONObject).filter { it.optString("type") == "config" }
    }
    private class Rig : AutoCloseable {
        val prefs = Prefs(RuntimeEnvironment.getApplication()).apply { checkUpdates = false; streamOverrides = StreamOverrides() }
        val sockets = mutableListOf<Socket>()
        val callbacks = mutableListOf<() -> Unit>()
        val capture = TouchCapture(WebSocket.Factory { _, listener -> Socket(listener).also { sockets.add(it) } })
        val session = SessionCoordinator(prefs, { callbacks.add(it) }, null, capture)
        fun start() { session.start(); sockets.last().open(); flush() }
        fun flush() { val batch = callbacks.toList(); callbacks.clear(); batch.forEach { it() } }
        override fun close() = session.stop()
    }

    @Test fun t1694HostObservationSeedsSettingsWithoutPersistingOrSendingDefaults() {
        Rig().use { rig ->
            rig.start()
            rig.sockets.last().greet(25005, 90); rig.flush()
            assertEquals(25005, rig.session.settings.bitrateKbps)
            assertEquals(90, rig.session.settings.fps)
            rig.session.handle(SettingsEvent.Brightness(70))
            assertEquals(25005, rig.session.settings.bitrateKbps)
            assertEquals(90, rig.session.settings.fps)
            assertTrue(rig.prefs.streamOverrides.empty)
            assertTrue(rig.sockets.last().configs().isEmpty())
        }
    }

    @Test fun t1694ClearCutsOffQueuedRejectionAndPreventsRestartReplay() {
        Rig().use { rig ->
            rig.start()
            val socket = rig.sockets.last()
            socket.greet(25005, 60); rig.flush()
            rig.session.handle(SettingsEvent.Stream(null, 90))
            socket.message(JSONObject().put("status", "settings_rejected").put("bitrate", 25005).put("fps", 60)
                .put("requested", JSONObject().put("fps", 90)))
            rig.session.handle(SettingsEvent.ClearStreamOverrides)
            rig.flush()
            assertTrue(rig.prefs.streamOverrides.empty)
            assertNull(rig.session.settings.streamError)
            assertEquals(60, rig.session.settings.fps)
            assertEquals(1, socket.configs().size)
            rig.session.stop(); rig.start()
            assertTrue(rig.sockets.last().configs().isEmpty())
        }
    }

    @Test fun t1694RejectedPartialEditRetainsOnlyConfirmedExplicitFields() {
        Rig().use { rig ->
            rig.start()
            rig.session.handle(SettingsEvent.Stream(null, 90))
            rig.sockets.last().message(JSONObject().put("status", "settings_rejected").put("bitrate", 25005).put("fps", 60)
                .put("requested", JSONObject().put("fps", 90)))
            rig.flush()
            assertEquals(StreamOverrides(fps = 60), rig.prefs.streamOverrides)
            assertEquals(25005, rig.session.settings.bitrateKbps)
            assertEquals(60, rig.session.settings.fps)
            assertEquals(HostStreamSettings(25005, 60), rig.capture.control.hostSettings.value!!.values)
            rig.session.stop(); rig.start()
            val replay = rig.sockets.last().configs().single()
            assertFalse(replay.has("bitrate"))
            assertEquals(60, replay.getInt("fps"))
        }
    }

    @Test fun t1694SupersededAndDisconnectedHostObservationsCannotReachUi() {
        Rig().use { rig ->
            rig.start()
            rig.sockets.last().greet(5000, 30)
            rig.sockets.last().greet(25005, 90)
            rig.flush()
            assertEquals(25005, rig.session.settings.bitrateKbps)
            rig.sockets.last().greet(6000, 60)
            rig.capture.disconnect(); rig.capture.connect()
            rig.flush()
            assertEquals(25005, rig.session.settings.bitrateKbps)
        }
    }
}
