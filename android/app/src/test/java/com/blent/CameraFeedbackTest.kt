package com.blent

import java.io.ByteArrayInputStream
import java.io.DataInputStream
import java.nio.ByteBuffer
import okio.Buffer
import okio.buffer
import okio.sink
import okio.source
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [27, 34])
class CameraFeedbackTest {
    @Test fun t614_feedbackRequiresExactConnectionLocalSequence() {
        val output = Buffer()
        val acks = Buffer().apply { for (i in 1L..31L) writeLong(i) }.readByteArray()
        val link = CameraLink(output, Buffer().write(acks))
        repeat(31) { assertTrue(link.send(ByteBuffer.wrap(byteArrayOf(7)), 0, 1, 0) >= 0) }
        repeat(31) { assertEquals(1, output.readInt()); assertEquals(7, output.readByte().toInt()) }
        for (ack in listOf(-1L, 0L, 2L, Long.MAX_VALUE)) {
            val input = Buffer().writeLong(ack).readByteArray()
            val invalid = CameraLink(Buffer(), Buffer().write(input))
            assertTrue(runCatching { invalid.send(ByteBuffer.wrap(byteArrayOf(1)), 0, 1, 0) }.isFailure)
        }
        for (length in 0..7) {
            val truncated = CameraLink(Buffer(), Buffer().write(ByteArray(length)))
            assertTrue(runCatching { truncated.send(ByteBuffer.wrap(byteArrayOf(1)), 0, 1, 0) }.isFailure)
        }
    }

    @Test fun t805_storageBorrowEndsAfterFlushAndNeverOnFailedTransport() {
        for (failWrite in listOf(false, true)) {
            var flushed = false
            var copied = 0
            var closed = 0
            val target = object : okio.ForwardingSink(Buffer()) {
                override fun flush() {
                    if (failWrite) throw java.io.IOException("failed flush")
                    flushed = true
                }
            }.buffer()
            val link = CameraLink(target, Buffer().writeLong(1), close = { closed++ })
            val result = runCatching { link.send(ByteBuffer.wrap(byteArrayOf(1)), 0, 1, 0) {
                assertTrue(flushed); copied++
            } }
            assertEquals(!failWrite, result.isSuccess)
            assertEquals(if (failWrite) 0 else 1, copied)
            assertEquals(if (failWrite) 1 else 0, closed)
            if (failWrite) assertThrows(IllegalStateException::class.java) { link.send(ByteBuffer.allocate(1), 0, 1, 0) }
        }
    }

    @Test fun t805_missingAckAndSocketStopKeepReleaseAndRetirementBounded() {
        for (stop in listOf(false, true)) {
            java.net.ServerSocket(0).use { server ->
                java.net.Socket("127.0.0.1", server.localPort).use { socket ->
                    server.accept().use { peer ->
                        val timer = java.util.concurrent.Executors.newSingleThreadScheduledExecutor()
                        var released = 0
                        var retired = 0
                        val link = CameraLink(socket.sink().buffer(), socket.source().buffer(), 50) { retired++; socket.close() }
                        val started = System.nanoTime()
                        try {
                            assertThrows(CameraTransportException::class.java) {
                                link.send(ByteBuffer.wrap(byteArrayOf(7)), 0, 1, 0) {
                                    released++
                                    if (stop) timer.schedule({ socket.close() }, 10, java.util.concurrent.TimeUnit.MILLISECONDS)
                                }
                            }
                            assertEquals(1, released); assertEquals(1, retired)
                            assertTrue("T805 missing ACK/Stop stuck", System.nanoTime() - started < 2_000_000_000L)
                            val data = java.io.DataInputStream(peer.getInputStream())
                            assertEquals(1, data.readInt()); assertEquals(7, data.read())
                            assertThrows(IllegalStateException::class.java) { link.send(ByteBuffer.allocate(1), 0, 1, 0) }
                        } finally { timer.shutdownNow() }
                    }
                }
            }
        }
    }

    @Test fun t614_relativeAgeExcludesClockOffsetAndRejectsRegression() {
        val clock = CameraFrameClock()
        assertEquals(0L, clock.ageUs(1000, 9_000_000))
        assertEquals(150_000L, clock.ageUs(2000, 9_151_000))
        assertEquals(0L, clock.ageUs(300_000, 9_200_000))
        assertTrue(runCatching { clock.ageUs(2000, 9_300_000) }.isFailure)
        assertTrue(runCatching { clock.ageUs(400_000, 8_000_000) }.isFailure)
        assertTrue(runCatching { CameraFrameClock().ageUs(-1, 0) }.isFailure)
        assertTrue(runCatching { CameraFrameClock().ageUs(0, -1) }.isFailure)
        assertEquals(0L, CameraFrameClock().ageUs(Long.MAX_VALUE, Long.MAX_VALUE))
    }
}
