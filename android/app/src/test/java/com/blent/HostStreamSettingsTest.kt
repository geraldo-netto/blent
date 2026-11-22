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
class HostStreamSettingsTest {
    private object Input : ControlInputState {
        override fun reset() {}
        override fun forgetTouches() {}
        override fun setTouchEnabled(enabled: Boolean) {}
        override fun setPenEnabled(enabled: Boolean) {}
    }
    private class Socket(val listener: WebSocketListener) : WebSocket {
        override fun request() = Request.Builder().url("ws://localhost/").build()
        override fun queueSize() = 0L
        override fun send(text: String) = true
        override fun send(bytes: ByteString) = true
        override fun close(code: Int, reason: String?) = true
        override fun cancel() {}
        fun greet(bitrate: Int, fps: Int) = listener.onMessage(this,
            JSONObject().put("bitrate", bitrate).put("fps", fps).toString())
    }
    private class Rig : AutoCloseable {
        val sockets = mutableListOf<Socket>()
        val control = ControlSession(Any(), Input, WebSocket.Factory { _, listener ->
            Socket(listener).also { sockets.add(it) }
        })
        fun connect(): Socket { control.connect(); return sockets.last() }
        override fun close() = control.disconnect()
    }

    @Test fun t1693PairedObservationsUseOnlyTheOriginalConnection() {
        Rig().use { rig ->
            assertNull(rig.control.hostSettings.value)
            val first = rig.connect()
            first.greet(9000, 30)
            val original = rig.control.hostSettings.value!!
            assertEquals(HostStreamSettings(9000, 30), original.values)
            assertTrue(rig.control.isCurrentConnection(original.source))
            first.listener.onMessage(first, "{}")
            assertSame(original, rig.control.hostSettings.value)
            rig.control.disconnect()
            assertNull(rig.control.hostSettings.value)
            assertFalse(rig.control.isCurrentConnection(original.source))
            val second = rig.connect()
            first.greet(5000, 90)
            assertNull(rig.control.hostSettings.value)
            second.greet(8000, 60)
            val replacement = rig.control.hostSettings.value!!
            assertEquals(HostStreamSettings(8000, 60), replacement.values)
            assertNotSame(original.source, replacement.source)
        }
    }

    @Test fun t1693MalformedRefreshCannotRetainAnApparentlyCurrentPair() {
        Rig().use { rig ->
            val socket = rig.connect()
            socket.greet(20000, 60)
            assertNotNull(rig.control.hostSettings.value)
            socket.listener.onMessage(socket, "{\"fps\":30}")
            assertNull(rig.control.hostSettings.value)
            socket.greet(25000, 90)
            assertEquals(25000, rig.control.hostSettings.value!!.values.bitrateKbps)
        }
    }

    @Test fun t1693CurrentPairedContractPreservesBoundaryValues() {
        for (bitrate in listOf(1000, 20000, 60000)) {
            for (fps in listOf(10, 60, 90)) {
                val message = JSONObject().put("bitrate", bitrate).put("fps", fps)
                assertEquals(HostStreamSettings(bitrate, fps), HostStreamSettingsJson.read(message))
            }
        }
        assertNull(HostStreamSettingsJson.read(JSONObject()))
        assertNull(HostStreamSettingsJson.read(JSONObject().put("bitrate", 20000)))
    }

    @Test fun t1693WireRejectsPartialCoercedOverflowAndOutOfRangeValues() {
        val bad = listOf(-1, 0, 1.5, "20", true, JSONObject.NULL, 2147483648L)
        for (field in listOf("bitrate", "fps")) {
            for (value in bad) {
                val message = JSONObject().put("bitrate", 20000).put("fps", 60).put(field, value)
                assertNull("T1693 $field=$value", HostStreamSettingsJson.read(message))
            }
        }
        for ((bitrate, fps) in listOf(999 to 60, 60001 to 60, 20000 to 9, 20000 to 91)) {
            assertNull(HostStreamSettingsJson.read(JSONObject().put("bitrate", bitrate).put("fps", fps)))
            assertThrows(IllegalArgumentException::class.java) { HostStreamSettings(bitrate, fps) }
        }
    }
}
