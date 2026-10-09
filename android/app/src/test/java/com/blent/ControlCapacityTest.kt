package com.blent

import okhttp3.*
import okio.ByteString
import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config
import java.util.ArrayDeque

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [27, 34])
class ControlCapacityTest {
    private class SlowSocket(val listener: WebSocketListener) : WebSocket {
        data class Queued(var bytes: Int, val time: Long, val type: String)
        val messages = ArrayDeque<Queued>()
        val ages = mutableMapOf<String, Long>()
        var now = 0L
        var bytes = 0L
        var peak = 0L
        var cancelled = false
        override fun request() = Request.Builder().url(TouchCapture.WS_URL).build()
        override fun queueSize() = bytes
        override fun send(text: String): Boolean {
            val count = text.toByteArray(Charsets.UTF_8).size
            messages.add(Queued(count, now, JSONObject(text).getString("type")))
            bytes += count; peak = maxOf(peak, bytes)
            return true // Transport never refuses: Blent must enforce capacity.
        }
        override fun send(bytes: ByteString) = false
        override fun close(code: Int, reason: String?) = true
        override fun cancel() { cancelled = true; messages.clear(); bytes = 0 }
        fun open() = listener.onOpen(this, Response.Builder().request(request()).protocol(Protocol.HTTP_1_1).code(101).message("test").build())
        fun drain(count: Int) {
            var left = count
            while (left > 0 && messages.isNotEmpty()) {
                val head = messages.first
                val amount = minOf(left, head.bytes)
                head.bytes -= amount; bytes -= amount; left -= amount
                if (head.bytes == 0) {
                    ages[head.type] = maxOf(ages[head.type] ?: 0, now - head.time)
                    messages.removeFirst()
                }
            }
        }
    }

    @Test fun t803_exactUtf8BudgetAndInvalidLimitsAreSafe() {
        lateinit var socket: SlowSocket
        val capture = TouchCapture(WebSocket.Factory { _, listener -> SlowSocket(listener).also { socket = it } })
        try {
            for (invalid in listOf(Int.MIN_VALUE, -1, 0, 8191, 1048577, Int.MAX_VALUE)) {
                assertThrows(IllegalArgumentException::class.java) { capture.setQueueBudget(invalid) }
            }
            capture.setQueueBudget(8192)
            capture.token = "a".repeat(64); capture.connect(); socket.open()
            val message = JSONObject().put("type", "test").put("value", "é😀")
            val size = message.toString().toByteArray(Charsets.UTF_8).size
            socket.bytes = 8192L - size
            capture.control.sendWhenConnected(message)
            assertEquals(8192, socket.bytes)
            assertTrue(capture.isControlConnected())
            capture.control.sendWhenConnected(message)
            assertTrue(socket.cancelled)
        } finally { capture.disconnect() }
    }

    @Test fun t803_retiredReceiptAndConnectionCannotSendAfterWaitingForControlLock() {
        lateinit var socket: SlowSocket
        val capture = TouchCapture(WebSocket.Factory { _, listener -> SlowSocket(listener).also { socket = it } })
        capture.token = "a".repeat(64)
        try {
            for (reconnect in listOf(false, true)) {
                capture.connect(); socket.open()
                val original = socket
                val current = java.util.concurrent.atomic.AtomicBoolean(true)
                val receipt = RenderAcknowledgement(17, 123, "old-decoder", current::get)
                val worker = Thread { capture.sendRendered(receipt) }
                synchronized(capture) {
                    worker.start()
                    val deadline = System.nanoTime() + java.util.concurrent.TimeUnit.SECONDS.toNanos(2)
                    while (worker.state != Thread.State.BLOCKED && System.nanoTime() < deadline) Thread.yield()
                    assertEquals(Thread.State.BLOCKED, worker.state)
                    if (reconnect) { capture.disconnect(); capture.connect(); socket.open() }
                    else current.set(false)
                }
                worker.join(2000)
                assertFalse(worker.isAlive)
                assertTrue(original.messages.none { it.type == "rendered" })
                assertTrue(socket.messages.none { it.type == "rendered" })
                capture.disconnect()
            }
        } finally { capture.disconnect() }
    }

    @Test fun t803_capacityArithmeticMatchesBoundedReference() {
        val random = java.util.Random(803)
        val cases = listOf(Long.MIN_VALUE, -1L, 0L, 1L, 8192L, 65536L, 1048576L, Long.MAX_VALUE)
        for (queued in cases + List(512) { random.nextLong() }) {
            for (incoming in cases) {
                for (limit in listOf(8191, 8192, 65536, 1048576, 1048577)) {
                    val expected = limit in 8192..1048576 && queued >= 0 && incoming > 0 &&
                        java.math.BigInteger.valueOf(queued).add(java.math.BigInteger.valueOf(incoming)) <= java.math.BigInteger.valueOf(limit.toLong())
                    assertEquals(expected, ControlCapacity.allows(queued, incoming, limit))
                }
            }
        }
    }

    @Test fun t803_acceptingSlowTransportCannotAccumulateUnboundedInputAndAcks() {
        lateinit var socket: SlowSocket
        val capture = TouchCapture(WebSocket.Factory { _, listener -> SlowSocket(listener).also { socket = it } })
        try {
            capture.token = "a".repeat(64); capture.connect(); socket.open()
            for (tick in 0 until 2000) {
                socket.now = tick * 10L
                socket.drain(100)
                for (kind in listOf("touch", "mouse", "rendered")) {
                    capture.control.sendWhenConnected(JSONObject().put("type", kind).put("sample", "x".repeat(100)), socket.now)
                }
                if (!capture.isControlConnected()) break
            }
            println("T803_SLOW peak_bytes=${socket.peak} retirement_ms=${socket.now} drained_age_ms=${socket.ages}")
            assertTrue("T803 accepting transport retained ${socket.peak} bytes", socket.peak <= 65_536)
            assertTrue(socket.cancelled)
            assertFalse(capture.isControlConnected())
            assertEquals(1, capture.controlStatistics().rejected)
        } finally { capture.disconnect() }
    }
}
