package com.blent

import java.io.ByteArrayOutputStream
import java.io.InputStream
import java.net.Socket
import java.net.SocketAddress
import java.net.SocketTimeoutException
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicBoolean
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.withTimeout
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config
import org.robolectric.util.ReflectionHelpers

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [27, 34])
class VideoConnectionBoundaryTest {
    @Test fun t497_periodicStatisticsStopWhenTheirSessionGenerationRetires() = runBlocking {
        val receiver = VideoReceiver { error("T497 idle sampler must not open a socket") }
        receiver.start() // No Surface: neither connection nor codec creation is admitted.
        val job: kotlinx.coroutines.Job = ReflectionHelpers.getField(receiver, "job")
        val statistics: ReceiverStatistics = ReflectionHelpers.getField(receiver, "statistics")
        try {
            repeat(30) { statistics.frameRendered() }
            statistics.bytesReceived(125_000)
            withTimeout(5000) {
                while (receiver.getFps() == 0f) kotlinx.coroutines.delay(10)
            }
            assertTrue(receiver.getMbps() > 0f)
            val workers = job.children.toList()
            assertEquals(2, workers.size)
            val generation: java.util.concurrent.atomic.AtomicLong = ReflectionHelpers.getField(receiver, "sessionGeneration")
            generation.incrementAndGet()
            // T799: the synthetic private generation change needs a lifecycle
            // wakeup; production Start/Stop publishes one without polling.
            receiver.onSurfaceDestroyed()
            val retainedFps = receiver.getFps()
            val retainedMbps = receiver.getMbps()
            withTimeout(5000) { workers.forEach { it.join() } }
            assertEquals("T497 retired sampler consumed another interval", retainedFps, receiver.getFps(), 0f)
            assertEquals(retainedMbps, receiver.getMbps(), 0f)
            assertFalse(job.isCancelled)
        } finally {
            receiver.stop()
            withTimeout(2000) { job.join() }
        }
    }

    @Test fun t497_defaultSocketFactoryDoesNotConnectBeforeTransportAdmission() {
        val connection = VideoReceiver().transport.newConnection()
        try {
            assertFalse(connection.isConnected)
            assertFalse(connection.isClosed)
        } finally { connection.close() }
        assertTrue(connection.isClosed)
    }

    @Test fun t497_invalidLengthRetiresAStreamWithoutWaitingForPayload() {
        val socket = object : Socket() {
            override fun connect(endpoint: SocketAddress?, timeout: Int) {}
            override fun setTcpNoDelay(value: Boolean) {}
            override fun setSoTimeout(value: Int) {}
            override fun setReceiveBufferSize(value: Int) {}
            override fun getInputStream(): InputStream = byteArrayOf(-1, -1, -1, -1).inputStream()
        }
        val receiver = VideoReceiver { socket }
        receiver.mimeType = "video/x-vnd.on2.vp9"
        val surfaceReady: AtomicBoolean = ReflectionHelpers.getField(receiver, "surfaceReady")
        surfaceReady.set(true)
        val disconnected = CountDownLatch(1)
        receiver.onDisconnected = { disconnected.countDown() }
        try {
            receiver.start()
            assertTrue("T497 invalid size did not terminate the stream", disconnected.await(2, TimeUnit.SECONDS))
            val job: kotlinx.coroutines.Job = ReflectionHelpers.getField(receiver, "job")
            receiver.stop()
            runBlocking { withTimeout(2000) { job.join() } }
            assertTrue(socket.isClosed)
        } finally { receiver.stop(); socket.close() }
    }

    private class FixtureSocket(private val mode: String) : Socket() {
        val authentication = ByteArrayOutputStream()
        var flushed = false
        override fun connect(endpoint: SocketAddress?, timeout: Int) { assertTrue(timeout > 0) }
        override fun setTcpNoDelay(value: Boolean) {}
        override fun setSoTimeout(value: Int) {}
        override fun setReceiveBufferSize(value: Int) {}
        override fun getOutputStream() = object : java.io.OutputStream() {
            override fun write(value: Int) { authentication.write(value) }
            override fun flush() { flushed = true }
        }
        override fun getInputStream(): InputStream = object : InputStream() {
            override fun read(): Int {
                assertTrue("T497 attempted video input before flushing authentication", flushed)
                if (mode == "timeout") throw SocketTimeoutException("T497 deadline")
                if (mode == "error") throw java.io.IOException("T497 reader failure")
                return -1
            }
        }
    }

    @Test fun t497_authenticatedReadFailuresRetireTheSocketAndPublishDisconnection() {
        for (mode in listOf("eof", "timeout", "error")) {
            val socket = FixtureSocket(mode)
            val receiver = VideoReceiver { socket }
            receiver.mimeType = "video/x-vnd.on2.vp9"
            receiver.token = "a".repeat(64)
            val surfaceReady: AtomicBoolean = ReflectionHelpers.getField(receiver, "surfaceReady")
            surfaceReady.set(true)
            val disconnected = CountDownLatch(1)
            receiver.onDisconnected = { disconnected.countDown() }
            try {
                receiver.start()
                assertTrue("T497 failed to publish disconnection", disconnected.await(2, TimeUnit.SECONDS))
                assertEquals("a".repeat(64), socket.authentication.toString("US-ASCII"))
                val job: kotlinx.coroutines.Job = ReflectionHelpers.getField(receiver, "job")
                receiver.stop()
                runBlocking { withTimeout(2000) { job.join() } }
                assertTrue(socket.isClosed)
            } finally { receiver.stop(); socket.close() }
        }
    }
    private class LivenessSocket : Socket() {
        val acknowledgement = ByteArrayOutputStream()
        override fun getOutputStream(): java.io.OutputStream = acknowledgement
        val consumed = CountDownLatch(1)
        private val released = CountDownLatch(1)
        val deadline = java.util.concurrent.atomic.AtomicInteger()
        private val bytes = ByteArray(6 * 12) { byteArrayOf(0, 0, 0, 2, 2, 1)[it % 6] }
        override fun connect(endpoint: SocketAddress?, timeout: Int) {}
        override fun setTcpNoDelay(value: Boolean) {}
        override fun setSoTimeout(value: Int) { deadline.set(value) }
        override fun setReceiveBufferSize(value: Int) {}
        override fun getInputStream(): InputStream = object : java.io.ByteArrayInputStream(bytes) {
            override fun read(data: ByteArray, offset: Int, size: Int): Int {
                if (available() != 0) return super.read(data, offset, size)
                consumed.countDown()
                check(released.await(5, TimeUnit.SECONDS))
                return -1
            }
        }
        override fun close() { super.close(); released.countDown() }
    }
    @Test fun t1038_livenessRetainsActualReceiverWithoutFrameRenderOrConnectionCallbacks() {
        val socket = LivenessSocket()
        val opens = java.util.concurrent.atomic.AtomicInteger()
        val connected = java.util.concurrent.atomic.AtomicInteger()
        val disconnected = java.util.concurrent.atomic.AtomicInteger()
        val rendered = java.util.concurrent.atomic.AtomicInteger()
        val receiver = VideoReceiver { opens.incrementAndGet(); socket }
        receiver.mimeType = "video/x-vnd.on2.vp9"
        receiver.setVideoLiveness(true)
        ReflectionHelpers.getField<AtomicBoolean>(receiver, "surfaceReady").set(true)
        receiver.onConnected = { connected.incrementAndGet() }
        receiver.onDisconnected = { disconnected.incrementAndGet() }
        receiver.onFrameRendered = { rendered.incrementAndGet() }
        try {
            receiver.start()
            assertTrue("T1038 actual receiver rejected liveness", socket.consumed.await(2, TimeUnit.SECONDS))
            assertArrayEquals("!BLENT1\n".toByteArray(Charsets.US_ASCII), socket.acknowledgement.toByteArray())
            assertEquals(10000, socket.deadline.get())
            assertEquals(1, opens.get())
            assertEquals(0, connected.get())
            assertEquals(0, disconnected.get())
            assertEquals(0, rendered.get())
            val job = ReflectionHelpers.getField<kotlinx.coroutines.Job>(receiver, "job")
            receiver.stop()
            runBlocking { withTimeout(2000) { job.join() } }
            assertTrue(socket.isClosed)
        } finally { receiver.stop(); socket.close() }
    }

}
