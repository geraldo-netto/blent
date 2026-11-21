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
class VideoPresenceTest {
    private open class Peer : Socket() {
        val bytes = ByteArrayOutputStream()
        override fun connect(endpoint: SocketAddress?, timeout: Int) {}
        override fun setTcpNoDelay(value: Boolean) {}
        override fun setSoTimeout(value: Int) {}
        override fun setReceiveBufferSize(value: Int) {}
        override fun getInputStream() = ByteArrayInputStream(byteArrayOf())
        override fun getOutputStream(): java.io.OutputStream = bytes
    }
    @Test fun t1039_currentWorkerRejectsEveryObsoleteFeatureValue() {
        val current = MediaTask.Display(AdbDisplayRoutes.create("a".repeat(64)), DecoderFormat("video/avc", 64, 64, 30))
        for (value in listOf<Any>(1, 0, 2, -1, Int.MAX_VALUE, 1.5, "1", true, false, JSONObject.NULL)) {
            val message = JSONObject(MediaTaskJson.encode(current)).put("liveness", value)
            assertTrue("$value", runCatching { MediaTaskJson.decode(message.toString()) }.isFailure)
        }
    }
    @Test fun t1039_originalSocketWritesOnlyCredentialsAndRetirementPreservesReplacement() {
        val current = AtomicLong(7)
        val peer = Peer()
        val transport = VideoTransport(Any(), { it == current.get() }, { peer })
        try {
            assertNotNull(transport.open(7, peer, AdbDisplayRoutes.create("a".repeat(64))))
            assertEquals("a".repeat(64), peer.bytes.toString("US-ASCII"))
            current.set(8)
            val next = Peer()
            assertNotNull(transport.open(8, next, AdbDisplayRoutes.create()))
            assertEquals(0, next.bytes.size())
            assertNull(transport.open(7, peer, AdbDisplayRoutes.create("a".repeat(64))))
            transport.retire(peer)
            assertFalse(next.isClosed)
            assertNotNull(transport.open(8, next, AdbDisplayRoutes.create()))
            assertEquals(0, next.bytes.size())
            transport.stop()
            assertTrue(next.isClosed)
        } finally { transport.stop(); peer.close() }
    }
    @Test fun t1039_cancelBlockedAuthenticationClosesOriginalWithoutGrantingAReplacement() {
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
        val thread = Thread { result.set(transport.open(1, peer, AdbDisplayRoutes.create("a".repeat(64)))) }
        try {
            thread.start()
            assertTrue(writing.await(2, TimeUnit.SECONDS))
            current.set(2)
            transport.stop()
            thread.join(2000)
            assertFalse(thread.isAlive)
            assertNull(result.get())
            val next = Peer()
            assertNotNull(transport.open(2, next, AdbDisplayRoutes.create()))
            assertEquals(0, next.bytes.size())
        } finally { transport.stop(); peer.close(); thread.join(2000) }
    }
    @Test fun t1039_workerSnapshotHasOneCurrentDisplayContract() {
        val task = MediaTask.Display(AdbDisplayRoutes.create("a".repeat(64)), DecoderFormat("video/avc", 64, 64, 30))
        val encoded = JSONObject(MediaTaskJson.encode(task))
        assertFalse(encoded.has("liveness"))
        assertEquals(task, MediaTaskJson.decode(encoded.toString()))
        for (value in listOf<Any>(0, 65536, -1, "8890", JSONObject.NULL, JSONObject())) {
            val invalid = JSONObject(encoded.toString()).put("videoPort", value)
            assertTrue(runCatching { MediaTaskJson.decode(invalid.toString()) }.isFailure)
        }
    }
    @Test fun t1039_receiverFormatChangesRetireOnlyItsOriginalWaitingRun() {
        val receiver = VideoReceiver { error("T1039 absent Surface must not open socket") }
        try {
            val original = DecoderFormat("video/avc", 64, 64, 30)
            val changed = original.copy(fps = 60)
            receiver.setStreamFormat(original)
            receiver.start()
            val previous = ReflectionHelpers.getField<Job>(receiver, "job")
            receiver.setStreamFormat(changed)
            assertTrue(previous.isCancelled)
            val replacement = ReflectionHelpers.getField<Job>(receiver, "job")
            assertNotSame(previous, replacement)
            receiver.setStreamFormat(changed)
            assertSame(replacement, ReflectionHelpers.getField<Job>(receiver, "job"))
            receiver.setStreamFormat(original)
            assertTrue(replacement.isCancelled)
        } finally { receiver.stop() }
    }
}
