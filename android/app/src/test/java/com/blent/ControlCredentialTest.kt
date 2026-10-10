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
class ControlCredentialTest {
    private class Socket(private val listener: WebSocketListener) : WebSocket {
        val sent = mutableListOf<String>()
        override fun request() = Request.Builder().url(TouchCapture.WS_URL).build()
        override fun queueSize() = 0L
        override fun send(text: String): Boolean { sent.add(text); return true }
        override fun send(bytes: ByteString) = false
        override fun close(code: Int, reason: String?) = true
        override fun cancel() {}
        fun open() = listener.onOpen(this, Response.Builder().request(request())
            .protocol(Protocol.HTTP_1_1).code(101).message("test").build())
        fun credentials() = sent.map(::JSONObject).filter { it.optString("type") == "auth" }.map { it.getString("token") }
    }
    private class Fixture {
        val sockets = mutableListOf<Socket>()
        val capture = TouchCapture(WebSocket.Factory { _, listener -> Socket(listener).also { sockets.add(it) } })
    }
    @Test fun t864PendingSocketNeverUsesReplacementCredential() {
        val fixture = Fixture(); val capture = fixture.capture
        val original = "a".repeat(64); val replacement = "b".repeat(64)
        try {
            capture.token = original; capture.connect()
            // Exact applyToken interleaving: publication precedes disconnect,
            // so the old current socket's asynchronous onOpen may run here.
            capture.token = replacement; fixture.sockets[0].open()
            assertEquals("T864: new credential crossed into old connection", listOf(original), fixture.sockets[0].credentials())
            capture.disconnect(); capture.connect(); fixture.sockets[1].open()
            assertEquals(listOf(replacement), fixture.sockets[1].credentials())
            val oldCount = fixture.sockets[0].sent.size
            fixture.sockets[0].open()
            assertEquals("T864: retired open callback sent authentication", oldCount, fixture.sockets[0].sent.size)
            assertEquals(listOf(replacement), fixture.sockets[1].credentials())
        } finally { capture.disconnect() }
    }
    @Test fun t864MissingStartupCredentialStaysAbsentOnItsAttempt() {
        val fixture = Fixture(); val capture = fixture.capture
        try {
            capture.connect(); capture.token = "b".repeat(64); fixture.sockets[0].open()
            assertTrue("T864: tokenless startup borrowed later credentials", fixture.sockets[0].credentials().isEmpty())
            capture.disconnect(); capture.connect(); fixture.sockets[1].open()
            assertEquals(listOf("b".repeat(64)), fixture.sockets[1].credentials())
        } finally { capture.disconnect() }
    }
    @Test fun t864NullAndMalformedUpdatesCannotChangePendingAuthentication() {
        for (candidate in listOf(null, "", "bad", "z".repeat(64), "a".repeat(65))) {
            val fixture = Fixture(); val capture = fixture.capture
            try {
                capture.token = "c".repeat(64); capture.connect(); capture.token = candidate; fixture.sockets[0].open()
                assertEquals(listOf("c".repeat(64)), fixture.sockets[0].credentials())
            } finally { capture.disconnect() }
        }
    }
}
