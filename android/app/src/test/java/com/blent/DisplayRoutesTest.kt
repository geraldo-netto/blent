package com.blent

import java.io.ByteArrayInputStream
import java.net.InetSocketAddress
import java.net.Socket
import java.net.SocketAddress
import java.util.concurrent.atomic.AtomicLong
import okhttp3.*
import okio.ByteString
import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config
import org.robolectric.util.ReflectionHelpers

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [27, 34])
class DisplayRoutesTest {
    private val token = "a".repeat(64)
    private fun route(revision: Long = 2) = DisplayRoutes(revision, token, 12001, 12002)
    private fun rejected(work: () -> Unit) { assertThrows(Exception::class.java) { work() } }
    @Test fun t856BoundsAndFuzzKeepDistinctLocalEndpoints() {
        for (revision in listOf(Long.MIN_VALUE, -1L, 0L)) rejected { route(revision) }
        for (port in listOf(Int.MIN_VALUE, -1, 0, 65536, Int.MAX_VALUE)) {
            rejected { route().copy(videoPort = port) }; rejected { route().copy(controlPort = port) }
        }
        rejected { route().copy(controlPort = 12001) }
        assertEquals(1, route(Long.MAX_VALUE).copy(videoPort = 1, controlPort = 65535).videoPort)
        val random = java.util.Random(856)
        repeat(2048) {
            val video = random.nextInt(100000) - 10000; val control = random.nextInt(100000) - 10000
            val valid = video in 1..65535 && control in 1..65535 && video != control
            assertEquals(valid, runCatching { DisplayRoutes(it.toLong() + 1, null, video, control) }.isSuccess)
        }
        assertFalse(route().toString().contains(token)); assertTrue(route().toString().contains("credential=true"))
        val adb = AdbDisplayRoutes.create(); assertEquals(8890, adb.videoPort); assertEquals(8891, adb.controlPort)
        assertEquals(1L, adb.revision); assertNull(adb.token); assertEquals("ws://127.0.0.1:8891", AdbDisplayRoutes.controlUrl)
    }
    @Test fun t856WorkerIpcPreservesWholeRouteAndRejectsCoercedNumbers() {
        val task = MediaTask.Display(route(), DecoderFormat("video/avc", 640, 480, 30))
        val encoded = MediaTaskJson.encode(task)
        assertEquals(task, MediaTaskJson.decode(encoded))
        for (field in listOf("routeRevision", "videoPort", "controlPort")) {
            for (invalid in listOf<Any>(-1, 0, "1234", 12.5, true, JSONObject.NULL, "1".repeat(100))) {
                rejected { MediaTaskJson.decode(JSONObject(encoded).put(field, invalid).toString()) }
            }
            rejected { MediaTaskJson.decode(JSONObject(encoded).apply { remove(field) }.toString()) }
        }
        for (port in listOf(65536L, Long.MAX_VALUE, 4294979297L)) {
            rejected { MediaTaskJson.decode(JSONObject(encoded).put("videoPort", port).toString()) }
        }
        rejected { MediaTaskJson.decode(JSONObject(encoded).put("controlPort", 12001).toString()) }
        rejected { MediaTaskJson.decode(JSONObject(encoded).put("token", "bad").toString()) }
        val large = MediaTask.Display(route(Long.MAX_VALUE), task.format)
        assertEquals(large, MediaTaskJson.decode(MediaTaskJson.encode(large)))
    }
    private class ControlSocket(private val request: Request, val listener: WebSocketListener) : WebSocket {
        val sent = mutableListOf<String>()
        override fun request() = request
        override fun queueSize() = 0L
        override fun send(text: String): Boolean { sent.add(text); return true }
        override fun send(bytes: ByteString) = false
        override fun close(code: Int, reason: String?) = true
        override fun cancel() {}
        fun open() = listener.onOpen(this, Response.Builder().request(request).protocol(Protocol.HTTP_1_1).code(101).message("test").build())
    }
    @Test fun t856ControlSnapshotsPortsAndCredentialsAndRejectsStaleOpen() {
        val sockets = mutableListOf<ControlSocket>()
        val capture = TouchCapture(WebSocket.Factory { request, listener -> ControlSocket(request, listener).also(sockets::add) })
        try {
            capture.routes = route(); assertEquals(route(), capture.routes); capture.connect()
            assertEquals(12002, sockets[0].request().url.port)
            capture.token = "b".repeat(64); sockets[0].open()
            assertEquals(token, JSONObject(sockets[0].sent.single()).getString("token"))
            val before = capture.connectionGeneration
            val replacement = DisplayRoutes(3, "c".repeat(64), 13001, 13002)
            capture.routes = replacement; assertTrue(capture.connectionGeneration > before)
            assertEquals(13002, sockets[1].request().url.port)
            val oldCount = sockets[0].sent.size; sockets[0].open(); assertEquals(oldCount, sockets[0].sent.size)
            sockets[1].open(); assertEquals(replacement.token, JSONObject(sockets[1].sent.single()).getString("token"))
            capture.routes = replacement; assertEquals(2, sockets.size)
            rejected { capture.routes = route() }; assertEquals(replacement, capture.routes)
            capture.routes = null; capture.routes = null; capture.token = token; capture.connect()
            assertNull(capture.routes); assertNull(capture.token); assertEquals(2, sockets.size)
        } finally { capture.disconnect() }
    }
    private class VideoSocket : Socket() {
        var destination: InetSocketAddress? = null
        override fun connect(endpoint: SocketAddress?, timeout: Int) { destination = endpoint as InetSocketAddress; assertEquals(5000, timeout) }
        override fun setTcpNoDelay(enabled: Boolean) { assertTrue(enabled) }
        override fun setSoTimeout(timeout: Int) { assertEquals(10000, timeout) }
        override fun setReceiveBufferSize(bytes: Int) { assertEquals(128 * 1024, bytes) }
        override fun getInputStream() = ByteArrayInputStream(byteArrayOf(1))
    }
    @Test fun t856VideoTransportUsesInjectedLocalRoute() {
        val socket = VideoSocket(); var current = true
        val transport = VideoTransport(Any(), { current }, { socket })
        try {
            assertSame(socket, transport.newConnection()); assertNotNull(transport.connect(1, socket, route()))
            assertEquals("127.0.0.1", socket.destination!!.address.hostAddress); assertEquals(12001, socket.destination!!.port)
            current = false; assertNull(transport.connect(1, socket, route(3)))
        } finally { transport.retire(socket); transport.stop() }
    }
    @Test fun t856VideoRouteChangesRetireGenerationWithoutImplicitFallback() {
        val receiver = VideoReceiver(initialRoutes = route()) { error("No surface; no socket should be admitted") }
        val generation: AtomicLong = ReflectionHelpers.getField(receiver, "sessionGeneration")
        try {
            assertEquals(route(), receiver.routes); assertEquals(token, receiver.token)
            receiver.start(); val before = generation.get()
            receiver.routes = route(3); assertTrue(generation.get() > before)
            val same = generation.get(); receiver.routes = route(3); assertEquals(same, generation.get())
            rejected { receiver.routes = route() }; assertEquals(route(3), receiver.routes)
            receiver.routes = null; val retired = generation.get(); receiver.routes = null
            receiver.token = token; receiver.start(); assertNull(receiver.routes); assertNull(receiver.token)
            assertEquals(retired, generation.get())
            receiver.routes = route(4); assertEquals(token, receiver.token)
        } finally { receiver.stop() }
        val absent = VideoReceiver(initialRoutes = null); absent.start(); assertNull(absent.routes); absent.stop()
    }
}
