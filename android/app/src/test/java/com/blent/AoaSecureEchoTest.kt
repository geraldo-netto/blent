package com.blent

import com.blent.connection.*
import java.io.*
import java.util.concurrent.CountDownLatch
import java.util.concurrent.Executors
import java.util.concurrent.TimeUnit
import java.util.concurrent.TimeoutException
import org.junit.Assert.*
import org.junit.Test

class AoaSecureEchoTest {
    private val finish = "BLENTUSB-FINISH".toByteArray() + byteArrayOf(1)
    private fun drain(mux: ConnectionMux): ByteArray {
        val bytes = ByteArrayOutputStream()
        while (true) {
            val frame = mux.next() ?: break
            val header = ByteArray(ConnectionWire.HEADER_BYTES)
            ConnectionWire.encode(frame.header, header)
            bytes.write(header); bytes.write(frame.bytes, frame.offset, frame.length)
            mux.finish(true)
        }
        return bytes.toByteArray()
    }
    private fun header(channel: Int, kind: Int, sequence: Long, length: Int, credit: Int = 0): ByteArray {
        val result = ByteArray(ConnectionWire.HEADER_BYTES)
        ConnectionWire.encode(ConnectionWire.Header(kind, channel, 7, 9, sequence, length, credit), result)
        return result
    }
    private fun prefix(frames: Int = 1): ByteArray {
        val mux = ConnectionMux(7, 9, 1048576, 64, 65536)
        val result = ByteArrayOutputStream().apply { write(drain(mux)) }
        repeat(frames) { sequence ->
            val length = intArrayOf(1, 64, 512, 4096, 16384, 65536)[(sequence / 6) % 6]
            result.write(header(sequence % 6 + 1, ConnectionWire.DATA, (sequence / 6 + 1).toLong(), length))
            result.write(ByteArray(length) { (it * 31 + sequence * 17).toByte() })
        }
        return result.toByteArray()
    }
    private fun receipt(channel: Int = 6, body: ByteArray = finish, sequence: Long = if (channel == 1) 2 else 1): ByteArray =
        header(channel, ConnectionWire.DATA, sequence, body.size) + body
    @Test fun t846FinalEchoCannotPassUntilAuthenticatedHostReceiptArrives() {
        val input = PipedInputStream(65536)
        val writer = PipedOutputStream(input)
        val executor = Executors.newSingleThreadExecutor()
        val consumed = CountDownLatch(1)
        val source = object : FilterInputStream(input) {
            var count = 0
            override fun read(bytes: ByteArray, offset: Int, length: Int): Int {
                val result = super.read(bytes, offset, length)
                count += result
                if (count >= prefix().size) consumed.countDown()
                return result
            }
        }
        val future = executor.submit<Long> { AoaSecureEcho(frames = 1).exchange(source, ByteArrayOutputStream()) }
        try {
            writer.write(prefix()); writer.flush()
            assertTrue(consumed.await(2, TimeUnit.SECONDS))
            assertThrows("T846: local final echo flush is not host acknowledgement", TimeoutException::class.java) {
                future.get(100, TimeUnit.MILLISECONDS)
            }
            writer.write(receipt()); writer.flush()
            assertEquals(1L, future.get(2, TimeUnit.SECONDS).toLong())
        } finally { writer.close(); input.close(); executor.shutdownNow(); assertTrue(executor.awaitTermination(3, TimeUnit.SECONDS)) }
    }
    @Test fun t846MissingMalformedAndWrongChannelReceiptsCannotPass() {
        val invalid = listOf(byteArrayOf(), receipt(body = byteArrayOf(1)), receipt(body = finish.copyOf().apply { this[lastIndex] = 2 }), receipt(channel = 2))
        for (ending in invalid) {
            assertThrows(Exception::class.java) {
                AoaSecureEcho(frames = 1).exchange(ByteArrayInputStream(prefix() + ending), ByteArrayOutputStream())
            }
        }
    }
    @Test fun t846FinalReceiptAllowsCreditAndRejectsExcessMetadata() {
        val credit = header(1, ConnectionWire.CREDIT, 2, 0, 1)
        assertEquals(1L, AoaSecureEcho(frames = 1).exchange(ByteArrayInputStream(prefix() + credit + receipt()), ByteArrayOutputStream()))
        val credits = ByteArrayOutputStream()
        repeat(6) { credits.write(header(1, ConnectionWire.CREDIT, it + 3L, 0, 1)) }
        assertEquals(70L, AoaSecureEcho(frames = 7).exchange(ByteArrayInputStream(prefix(7) + credits.toByteArray() + receipt(sequence = 2)), ByteArrayOutputStream()))
        credits.write(header(1, ConnectionWire.CREDIT, 9, 0, 1))
        val error = assertThrows(IllegalStateException::class.java) {
            AoaSecureEcho(frames = 7).exchange(ByteArrayInputStream(prefix(7) + credits.toByteArray() + receipt(sequence = 2)), ByteArrayOutputStream())
        }
        assertEquals("Host completion receipt exceeded metadata bound", error.message)
        for (frames in listOf(0, -1, 4097, Int.MAX_VALUE)) assertThrows(IllegalArgumentException::class.java) { AoaSecureEcho(frames) }
    }
}
