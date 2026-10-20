package com.blent

import kotlinx.coroutines.*
import okhttp3.*
import okio.ByteString
import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config
import java.util.concurrent.CopyOnWriteArrayList
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [27, 34])
class DecoderNegotiationTest {
    @Test fun t808_stopAndReconnectDoNotWaitOrPublishRetiredNativeResult() {
        val sockets = CopyOnWriteArrayList<Socket>()
        val entered = CountDownLatch(1)
        val release = CountDownLatch(1)
        val control = ControlSession(Any(), Input, WebSocket.Factory { _, listener -> Socket(listener).also { sockets.add(it) } }) {
            width, height, fps -> DecoderCapabilities.report(width, height, fps, {
                entered.countDown(); release.await(); emptyArray<android.media.MediaCodecInfo>()
            }, { _, _ -> error("empty") })
        }
        try {
            control.connect(); sockets[0].open(); sockets[0].greeting()
            assertTrue(entered.await(2, TimeUnit.SECONDS))
            control.disconnect()
            assertFalse(control.controlConnected.value)
            control.connect(); sockets[1].open(); sockets[1].greeting(1280)
            val deadline = System.nanoTime() + TimeUnit.SECONDS.toNanos(2)
            while (sockets[1].sent.none { it.contains("capabilities") } && System.nanoTime() < deadline) Thread.sleep(5)
            assertTrue(sockets[0].sent.none { it.contains("capabilities") })
            val report = JSONObject(sockets[1].sent.single { it.contains("capabilities") }).getJSONObject("capabilities")
            assertEquals(1280, report.getInt("width"))
            assertEquals("unavailable", report.getString("status"))
            release.countDown()
            runBlocking { withTimeout(2_000) { while (MetadataQueries.lane.busy) delay(1) } }
            assertTrue(sockets[0].sent.none { it.contains("capabilities") })
        } finally {
            release.countDown(); control.disconnect()
            runBlocking { withTimeout(2_000) { while (MetadataQueries.lane.busy) delay(1) } }
        }
    }

    @Test fun t478_lateInventoryCannotPublishAcrossIdenticalFormatScopes() {
        lateinit var socket: Socket
        val entered = CountDownLatch(1)
        val release = CountDownLatch(1)
        val queries = java.util.concurrent.atomic.AtomicInteger()
        val control = ControlSession(Any(), Input, WebSocket.Factory { _, listener -> Socket(listener).also { socket = it } }) {
                width, height, fps ->
            if (queries.getAndIncrement() == 0) { entered.countDown(); release.await(2, TimeUnit.SECONDS) }
            DecoderCapabilities.describe(width, height, fps) { _, _, _, _ -> true }.put("details", org.json.JSONArray())
        }
        fun greet(scope: String) = socket.listener.onMessage(socket,
            """{"decoder_protocol":2,"decoder_scope":"$scope","codec":"h264","fps":60,"video_width":640,"video_height":400}""")
        try {
            control.connect(); socket.open(); greet("7")
            assertTrue(entered.await(2, TimeUnit.SECONDS))
            greet("9"); release.countDown()
            val deadline = System.nanoTime() + TimeUnit.SECONDS.toNanos(2)
            while (socket.sent.none { it.contains("capabilities") } && System.nanoTime() < deadline) Thread.sleep(10)
            val reports = socket.sent.filter { it.contains("capabilities") }
            assertEquals(1, reports.size)
            assertEquals("9", JSONObject(reports.single()).getJSONObject("capabilities").getString("scope"))
        } finally { release.countDown(); control.disconnect() }
    }

    @Test fun t478_currentProtocolEchoesEachScopeAndRetainsDetails() {
        lateinit var socket: Socket
        val control = ControlSession(Any(), Input, WebSocket.Factory { _, listener -> Socket(listener).also { socket = it } }) {
                width, height, fps ->
            DecoderCapabilities.describe(width, height, fps) { _, _, _, _ -> true }
                .put("details", org.json.JSONArray())
        }
        fun receive(): JSONObject {
            val deadline = System.nanoTime() + TimeUnit.SECONDS.toNanos(2)
            while (socket.sent.none { it.contains("capabilities") } && System.nanoTime() < deadline) Thread.sleep(10)
            return JSONObject(socket.sent.single { it.contains("capabilities") }).getJSONObject("capabilities")
        }
        try {
            control.connect(); socket.open(); socket.greeting()
            val initial = receive()
            assertEquals(2, initial.getInt("protocol"))
            assertEquals("7", initial.getString("scope"))
            assertTrue(initial.has("details"))
            assertTrue(initial.has("software"))
            socket.sent.clear()
            socket.listener.onMessage(socket, """{"decoder_protocol":2,"decoder_scope":"9","codec":"h264","fps":60,"video_width":640,"video_height":400}""")
            val rich = receive()
            assertEquals(2, rich.getInt("protocol"))
            assertEquals("9", rich.getString("scope"))
            assertTrue(rich.has("details"))
            assertTrue("T480: firmware/app software key missing", rich.getString("software").matches(Regex("[0-9a-f]{64}")))
        } finally { control.disconnect() }
    }

    private class Socket(val listener: WebSocketListener) : WebSocket {
        val sent = CopyOnWriteArrayList<String>()
        override fun request() = Request.Builder().url("ws://localhost/").build()
        override fun queueSize() = 0L
        override fun send(text: String): Boolean { sent.add(text); return true }
        override fun send(bytes: ByteString) = false
        override fun close(code: Int, reason: String?) = true
        override fun cancel() {}
        fun open() = listener.onOpen(this, Response.Builder().request(request()).protocol(Protocol.HTTP_1_1).code(101).message("test").build())
        fun greeting(width: Int = 640) = listener.onMessage(this,
            """{"status":"connected","decoder_protocol":2,"decoder_scope":"7","codec":"h264","fps":60,"video_width":$width,"video_height":400}""")
    }
    private object Input : ControlInputState {
        override fun reset() {}
        override fun forgetTouches() {}
        override fun setTouchEnabled(enabled: Boolean) {}
        override fun setPenEnabled(enabled: Boolean) {}
    }
    @Test fun t332_lateRejectionCannotReplaceANewerRequest() {
        lateinit var socket: Socket
        val control = ControlSession(Any(), Input, WebSocket.Factory { _, listener ->
            Socket(listener).also { socket = it }
        })
        var rejected = 0
        var reportedFps = 0
        control.onSettingsRejected = { rejected++ }
        control.onFpsKnown = { reportedFps = it }
        try {
            control.connect(); socket.open()
            control.sendConfig(25000, 90)
            control.sendConfig(15000, 30)
            val reply = JSONObject(javaClass.getResource("/settings-rejected.json")!!.readText())
                .put("requested", JSONObject().put("bitrate", 25000).put("fps", 90))
            socket.listener.onMessage(socket, reply.toString())
            assertEquals("T332: old rejection replaced newer edit", 0, rejected)
            assertEquals(0, reportedFps)
            control.disconnect(); control.connect(); socket.open()
            val resent = JSONObject(socket.sent.last())
            assertEquals(15000, resent.getInt("bitrate"))
            assertEquals(30, resent.getInt("fps"))
        } finally { control.disconnect() }
    }

    @Test fun t276_sharedScaledGreetingUsesEncodedDimensionsForCapabilities() {
        lateinit var socket: Socket
        val requested = java.util.concurrent.LinkedBlockingQueue<Triple<Int, Int, Int>>()
        val control = ControlSession(Any(), Input, WebSocket.Factory { _, listener ->
            Socket(listener).also { socket = it }
        }) { width, height, fps ->
            requested.put(Triple(width, height, fps))
            DecoderCapabilities.describe(width, height, fps) { _, _, _, _ -> true }
        }
        try {
            control.connect()
            socket.open()
            socket.listener.onMessage(socket, javaClass.getResource("/control-scaled.json")!!.readText())
            assertEquals(Triple(640, 400, 30), requested.poll(2, TimeUnit.SECONDS))
            assertTrue(control.controlConnected.value)
            assertFalse(control.isPenOnly)
        } finally { control.disconnect() }
    }

    @Test fun t432_staleCapabilityQueryCannotPublishAfterReconnect() {
        val sockets = CopyOnWriteArrayList<Socket>()
        val entered = CountDownLatch(1)
        val release = CompletableDeferred<Unit>()
        val control = ControlSession(Any(), Input, WebSocket.Factory { _, listener ->
            Socket(listener).also { sockets.add(it) }
        }) { width, height, fps ->
            entered.countDown()
            release.await()
            DecoderCapabilities.describe(width, height, fps) { _, _, _, _ -> true }
        }
        try {
            control.token = "test-token"
            control.connect()
            sockets[0].open()
            sockets[0].greeting()
            assertTrue(entered.await(2, TimeUnit.SECONDS))
            control.disconnect()
            control.connect()
            sockets[1].open()
            release.complete(Unit)
            // Current request runs after the retired request is cancelled.
            sockets[1].greeting(1280)
            val deadline = System.nanoTime() + TimeUnit.SECONDS.toNanos(2)
            while (sockets[1].sent.none { it.contains("capabilities") } && System.nanoTime() < deadline) Thread.sleep(10)
            assertTrue(sockets[0].sent.none { it.contains("capabilities") })
            val message = JSONObject(sockets[1].sent.single { it.contains("capabilities") })
            assertEquals(1280, message.getJSONObject("capabilities").getInt("width"))
            assertEquals("auth", JSONObject(sockets[1].sent.first()).getString("type"))
        } finally { release.complete(Unit); control.disconnect() }
    }
    @Test fun t1199MissingOrUnsupportedOfferDoesNotQueryOrDowngrade() {
        lateinit var socket: Socket
        val queried = CountDownLatch(1)
        val control = ControlSession(Any(), Input, WebSocket.Factory { _, listener -> Socket(listener).also { socket = it } }) {
            width, height, fps -> queried.countDown(); DecoderCapabilities.describe(width, height, fps) { _, _, _, _ -> true }
        }
        val invalid = listOf("{}", "{\"decoder_protocol\":1,\"decoder_scope\":\"7\"}",
            "{\"decoder_protocol\":2}", "{\"decoder_protocol\":2,\"decoder_scope\":\"\"}",
            "{\"decoder_protocol\":3,\"decoder_scope\":\"7\"}", "{\"decoder_protocol\":\"2\",\"decoder_scope\":\"7\"}",
            "{\"decoder_protocol\":2,\"decoder_scope\":7}")
        try {
            control.connect(); socket.open()
            for (offer in invalid) {
                val greeting = JSONObject(offer).put("status", "connected").put("codec", "h264")
                    .put("fps", 60).put("video_width", 640).put("video_height", 400)
                socket.listener.onMessage(socket, greeting.toString())
                assertFalse("T1199 unsupported offer admitted inventory: $offer", queried.await(200, TimeUnit.MILLISECONDS))
                assertTrue(socket.sent.none { it.contains("capabilities") })
            }
        } finally { control.disconnect() }
    }

    @Test fun t1199WithdrawnOfferCannotPublishHeldOriginalInventory() {
        lateinit var socket: Socket
        val entered = CountDownLatch(1)
        val release = CompletableDeferred<Unit>()
        val completed = CountDownLatch(1)
        val control = ControlSession(Any(), Input, WebSocket.Factory { _, listener -> Socket(listener).also { socket = it } }) {
            width, height, fps ->
            try { entered.countDown(); release.await(); DecoderCapabilities.describe(width, height, fps) { _, _, _, _ -> true } }
            finally { completed.countDown() }
        }
        try {
            control.connect(); socket.open()
            socket.listener.onMessage(socket, """{"decoder_protocol":2,"decoder_scope":"7","codec":"h264","fps":60,"video_width":640,"video_height":400}""")
            assertTrue(entered.await(2, TimeUnit.SECONDS))
            socket.listener.onMessage(socket, """{"codec":"h264","fps":60,"video_width":640,"video_height":400}""")
            release.complete(Unit)
            assertTrue(completed.await(2, TimeUnit.SECONDS))
            Thread.sleep(100)
            assertTrue("T1199 withdrawn scope published inventory", socket.sent.none { it.contains("capabilities") })
        } finally { release.complete(Unit); control.disconnect() }
    }

}
