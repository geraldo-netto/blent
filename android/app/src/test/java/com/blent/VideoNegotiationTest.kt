package com.blent

import java.io.ByteArrayInputStream
import java.io.ByteArrayOutputStream
import java.net.Socket
import java.net.SocketAddress
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicLong
import kotlinx.coroutines.Job
import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config
import org.robolectric.util.ReflectionHelpers

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [27, 34])
class VideoNegotiationTest {
    private open class Peer : Socket() {
        val bytes = ByteArrayOutputStream()
        override fun connect(endpoint: SocketAddress?, timeout: Int) {}
        override fun setTcpNoDelay(value: Boolean) {}
        override fun setSoTimeout(value: Int) {}
        override fun setReceiveBufferSize(value: Int) {}
        override fun getInputStream() = ByteArrayInputStream(byteArrayOf())
        override fun getOutputStream(): java.io.OutputStream = bytes
    }
    @Test fun t1039_onlyExplicitValidConnectedOfferEnablesAcknowledgement() {
        assertFalse(VideoNegotiation.offered(JSONObject()))
        for (status in listOf("connected", "mode", "", "settings_rejected")) {
            for (value in listOf<Any>(1, 0, 2, -1, Int.MAX_VALUE, 1.5, "1", true, JSONObject.NULL)) {
                val message = JSONObject().put("status", status).put("video_liveness", value)
                assertEquals("$status $value", status == "connected" && value == 1, VideoNegotiation.offered(message))
            }
        }
        assertArrayEquals(byteArrayOf(33, 66, 76, 69, 78, 84, 49, 10), VideoNegotiation.acknowledgement())
    }
    @Test fun t1039_originalSocketAuthenticatesBeforeAckAndLegacyHasNoNewPrelude() {
        val current = AtomicLong(7)
        val peer = Peer()
        val transport = VideoTransport(Any(), { it == current.get() }, { peer })
        try {
            assertNotNull(transport.open(7, peer, AdbDisplayRoutes.create("a".repeat(64)), true))
            assertEquals("a".repeat(64) + "!BLENT1\n", peer.bytes.toString("US-ASCII"))
            current.set(8)
            val next = Peer()
            assertNotNull(transport.open(8, next, AdbDisplayRoutes.create(), false))
            assertEquals(0, next.bytes.size())
            assertFalse(transport.acknowledge(7, peer, true))
            transport.retire(peer)
            assertFalse(next.isClosed)
            assertTrue(transport.acknowledge(8, next, false))
            assertEquals(0, next.bytes.size())
            transport.stop()
            assertTrue(next.isClosed)
        } finally { transport.stop(); peer.close() }
    }
    @Test fun t1039_cancelBlockedAckClosesOriginalWithoutGrantingAReplacement() {
        val writing = CountDownLatch(1)
        val released = CountDownLatch(1)
        val peer = object : Peer() {
            override fun getOutputStream() = object : java.io.OutputStream() {
                override fun write(value: Int) {
                    writing.countDown()
                    check(released.await(5, TimeUnit.SECONDS))
                }
            }
            override fun close() { super.close(); released.countDown() }
        }
        val current = AtomicLong(1)
        val transport = VideoTransport(Any(), { it == current.get() }, { peer })
        val result = java.util.concurrent.atomic.AtomicReference<java.io.InputStream?>()
        val thread = Thread { result.set(transport.open(1, peer, AdbDisplayRoutes.create(), true)) }
        try {
            thread.start()
            assertTrue(writing.await(2, TimeUnit.SECONDS))
            current.set(2)
            transport.stop()
            thread.join(2000)
            assertFalse(thread.isAlive)
            assertNull(result.get())
            val next = Peer()
            assertNotNull(transport.open(2, next, AdbDisplayRoutes.create(), false))
            assertEquals(0, next.bytes.size())
        } finally { transport.stop(); peer.close(); thread.join(2000) }
    }
    @Test fun t1039_workerSnapshotBoundsFeatureAndAbsenceRemainsLegacy() {
        val legacy = MediaTask.Display(AdbDisplayRoutes.create("a".repeat(64)), DecoderFormat("video/avc", 64, 64, 30))
        val enabled = legacy.copy(liveness = true)
        assertEquals(enabled, MediaTaskJson.decode(MediaTaskJson.encode(enabled)))
        val encoded = JSONObject(MediaTaskJson.encode(legacy)).apply { remove("liveness") }
        assertEquals(legacy, MediaTaskJson.decode(encoded.toString()))
        for (value in listOf<Any>(0, 1, -1, "true", JSONObject.NULL, JSONObject())) {
            val invalid = JSONObject(encoded.toString()).put("liveness", value)
            assertTrue(runCatching { MediaTaskJson.decode(invalid.toString()) }.isFailure)
        }
    }
    @Test fun t1039_receiverCapabilityChangesRetireOnlyItsOriginalWaitingRun() {
        val receiver = VideoReceiver { error("T1039 absent Surface must not open socket") }
        try {
            receiver.setVideoLiveness(false)
            receiver.start()
            val previous = ReflectionHelpers.getField<Job>(receiver, "job")
            receiver.setVideoLiveness(true)
            assertTrue(previous.isCancelled)
            val replacement = ReflectionHelpers.getField<Job>(receiver, "job")
            assertNotSame(previous, replacement)
            receiver.setVideoLiveness(true)
            assertSame(replacement, ReflectionHelpers.getField<Job>(receiver, "job"))
            receiver.setVideoLiveness(false)
            assertTrue(replacement.isCancelled)
        } finally { receiver.stop() }
    }
}
