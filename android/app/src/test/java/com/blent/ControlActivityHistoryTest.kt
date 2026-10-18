package com.blent

import com.blent.activity.DisplayActivityState
import kotlinx.coroutines.*
import okhttp3.*
import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config
import org.robolectric.util.ReflectionHelpers

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [27, 34])
class ControlActivityHistoryTest {
    private class Rig : AutoCloseable {
        val sockets = ArrayList<Pair<DisplayRouteSocket, WebSocketListener>>()
        val control = TouchCapture(WebSocket.Factory { request, listener ->
            DisplayRouteSocket(request).also { sockets.add(it to listener) }
        }, { _, _, _ -> JSONObject() })
        init { control.token = "ab".repeat(32) }
        fun open() {
            control.connect()
            val (socket, listener) = sockets.last()
            listener.onOpen(socket, Response.Builder().request(socket.request()).protocol(Protocol.HTTP_1_1).code(101).message("T1067 original source").build())
        }
        fun message(value: JSONObject) { val (socket, listener) = sockets.last(); listener.onMessage(socket, value.toString()) }
        override fun close() {
            control.disconnect()
            val scope = ReflectionHelpers.getField<CoroutineScope>(control.control, "scope")
            runBlocking { checkNotNull(scope.coroutineContext[Job]).cancelAndJoin() }
        }
    }
    private fun greeting(signal: Any? = null) = JSONObject().put("status", "connected").put("pen_only", false).apply {
        if (signal != null) put("display_activity", signal)
    }
    private fun active() = JSONObject().put("version", 1).put("revision", 1).put("stopped", 0).put("active", true)
    @Test fun t1067ReplacementCannotEraseKnownLossBeforeAnyUiDelivery() {
        Rig().use { rig ->
            rig.open(); rig.message(greeting(active()))
            val original = rig.control.displayActivityUpdate
            assertTrue(original.supportKnown)
            rig.control.disconnect(); rig.open()
            val latest = rig.control.displayActivityUpdate
            assertNotSame(original.source, latest.source)
            assertSame(DisplayActivityState.Unavailable, latest.state)
            assertTrue("T1067 replacement erased supported-source loss", latest.supportKnown)
            val (old, listener) = rig.sockets.first()
            listener.onMessage(old, greeting().toString())
            assertSame(latest, rig.control.displayActivityUpdate)
            rig.message(greeting())
            assertTrue(rig.control.connectionState.value.authenticated)
            assertSame(DisplayActivityState.Unsupported, rig.control.displayActivityUpdate.state)
            assertFalse(rig.control.displayActivityUpdate.supportKnown)
        }
    }
    @Test fun t1068ControlKeepsOnePureReaderThroughOriginalSourceReplacement() {
        Rig().use { rig ->
            val view = rig.control.control.displayActivityView
            rig.open(); rig.message(greeting(active()))
            val original = view.update
            assertSame(original, rig.control.displayActivityUpdate)
            assertTrue(view.current(original))
            rig.control.disconnect(); rig.open()
            assertSame(view, rig.control.control.displayActivityView)
            assertFalse(view.current(original))
            assertSame(view.update, rig.control.displayActivityUpdate)
            assertTrue(view.current(view.update))
        }
    }
    @Test fun t1067MalformedAnnouncementAndObserverFaultRetainLossAcrossReplacement() {
        Rig().use { rig ->
            rig.control.onDisplayActivityKnown = { error("T1067 UI observer unavailable") }
            rig.open(); rig.message(greeting("malformed"))
            assertFalse(rig.control.connectionState.value.authenticated)
            assertTrue(rig.sockets.single().first.cancelled)
            assertTrue(rig.control.displayActivityUpdate.supportKnown)
            rig.open()
            assertTrue("T1067 observer failure allowed initial-waiting downgrade", rig.control.displayActivityUpdate.supportKnown)
            assertSame(DisplayActivityState.Unavailable, rig.control.displayActivityUpdate.state)
            rig.message(greeting(active()))
            assertTrue(rig.control.connectionState.value.authenticated)
            assertTrue(rig.control.displayActivityUpdate.supportKnown)
        }
    }
}
