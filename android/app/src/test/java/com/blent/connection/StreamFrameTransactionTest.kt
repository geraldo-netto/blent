package com.blent.connection

import java.io.ByteArrayInputStream
import java.io.ByteArrayOutputStream
import java.io.Closeable
import java.io.IOException
import java.io.OutputStream
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import org.junit.Assert.*
import org.junit.Test

/** T1759 native transaction: exact borrowed slices, one flush, original exit fence. */
class StreamFrameTransactionTest {
    private fun await(condition: () -> Boolean) {
        val deadline = System.nanoTime() + TimeUnit.SECONDS.toNanos(3)
        while (!condition()) {
            check(System.nanoTime() < deadline) { "T1759 native worker did not settle" }
            Thread.sleep(1)
        }
    }
    private fun completion(io: ConnectionIo): ConnectionIoCompletion {
        var result: ConnectionIoCompletion? = null
        await { result = io.poll(); result != null }
        return checkNotNull(result)
    }
    private fun retire(io: ConnectionIo) { io.stop(); await(io::retired) }
    private fun transaction(payload: Int = ConnectionWire.MAX_PAYLOAD): ConnectionIoRequest.WriteFlushed {
        val header = ConnectionBuffer(ByteArray(42) { 17 }, 1, 40)
        val body = if (payload == 0) null else ConnectionBuffer(ByteArray(payload + 4) { 91 }, 2, payload)
        return ConnectionIoRequest.WriteFlushed(ConnectionWriteTransaction(header, body))
    }
    private class ObservedOutput : OutputStream() {
        val slices = mutableListOf<ConnectionBuffer>()
        val threads = mutableListOf<Thread>()
        val bytes = ByteArrayOutputStream()
        var flushes = 0
        override fun write(value: Int) { error("Unexpected single-byte write") }
        override fun write(source: ByteArray, offset: Int, length: Int) {
            slices.add(ConnectionBuffer(source, offset, length)); threads.add(Thread.currentThread())
            bytes.write(source, offset, length)
        }
        override fun flush() { flushes++; threads.add(Thread.currentThread()) }
    }
    private fun assertSlices(request: ConnectionIoRequest.WriteFlushed, output: ObservedOutput) {
        val expected = listOfNotNull(request.loan.first, request.loan.second)
        assertEquals(expected.size, output.slices.size)
        expected.zip(output.slices).forEach { (original, seen) ->
            assertSame(original.bytes, seen.bytes)
            assertEquals(original.offset, seen.offset); assertEquals(original.length, seen.length)
        }
        val bytes = expected.flatMap { it.bytes.copyOfRange(it.offset, it.offset + it.length).asList() }.toByteArray()
        assertArrayEquals(bytes, output.bytes.toByteArray())
    }
    @Test fun t1759NativeWorkerWritesOriginalSlicesAndFlushesBeforeSingleCompletion() {
        for (payload in listOf(0, 1, 16384, ConnectionWire.MAX_PAYLOAD)) {
            val output = ObservedOutput()
            val io = StreamConnectionIo(ByteArrayInputStream(byteArrayOf()), output)
            try {
                val request = transaction(payload)
                assertTrue(io.submit(request))
                val result = completion(io)
                assertSame(request, result.request); assertNull(result.failure)
                assertEquals(40 + payload, result.count); assertEquals(1, output.flushes)
                assertSlices(request, output)
                assertEquals(1, output.threads.distinct().size)
                assertNotSame(Thread.currentThread(), output.threads.first())
                assertNull(io.poll())
            } finally { retire(io) }
        }
    }
    private class GatedOutput(private val stage: Int, private val fail: Boolean) : OutputStream() {
        val entered = CountDownLatch(1)
        val release = CountDownLatch(1)
        var calls = 0
        override fun write(value: Int) { error("Unexpected single-byte write") }
        override fun write(bytes: ByteArray, offset: Int, length: Int) = step()
        override fun flush() = step()
        private fun step() {
            if (calls++ != stage) return
            entered.countDown()
            check(release.await(3, TimeUnit.SECONDS)) { "T1759 test did not release native operation" }
            if (fail) throw IOException("stage $stage")
        }
    }
    private fun entered(output: GatedOutput) { check(output.entered.await(3, TimeUnit.SECONDS)) }
    @Test fun t1759EveryNativeStageRetainsSlotAndOwnerThroughCancellation() {
        for (stage in 0..2) {
            val output = GatedOutput(stage, false)
            val closed = CountDownLatch(1)
            val io = StreamConnectionIo(ByteArrayInputStream(byteArrayOf()), output, Closeable { closed.countDown() })
            try {
                val request = transaction(3); assertTrue(io.submit(request)); entered(output)
                assertNull(io.poll()); assertFalse(io.submit(transaction(1)))
                io.stop(); check(closed.await(3, TimeUnit.SECONDS))
                assertFalse(io.retired()); assertFalse(io.submit(transaction(1)))
                output.release.countDown()
                val result = completion(io)
                assertSame(request, result.request); assertNull(result.failure); assertEquals(43, result.count)
                await(io::retired); assertEquals(3, output.calls)
            } finally { output.release.countDown(); retire(io) }
        }
    }
    @Test fun t1759FaultAtHeaderPayloadOrFlushNeverAcknowledgesFrameSuccess() {
        for (stage in 0..2) {
            val output = GatedOutput(stage, true)
            val io = StreamConnectionIo(ByteArrayInputStream(byteArrayOf()), output)
            try {
                val request = transaction(3); assertTrue(io.submit(request)); entered(output)
                assertNull(io.poll()); assertFalse(io.submit(transaction(1)))
                output.release.countDown()
                val result = completion(io)
                assertSame(request, result.request); assertEquals(0, result.count)
                assertEquals("stage $stage", result.failure?.message); assertEquals(stage + 1, output.calls)
                assertNull(io.poll())
            } finally { output.release.countDown(); retire(io) }
        }
    }
    private fun reject(io: ConnectionIo, first: ConnectionBuffer, second: ConnectionBuffer? = null) {
        assertThrows(IllegalArgumentException::class.java) {
            io.submit(ConnectionIoRequest.WriteFlushed(ConnectionWriteTransaction(first, second)))
        }
        assertNull(io.poll())
    }
    @Test fun t1759InvalidSlicesAndCombinedBoundsRejectBeforeAnyNativeAccess() {
        val output = ObservedOutput()
        val io = StreamConnectionIo(ByteArrayInputStream(byteArrayOf()), output)
        try {
            val good = ConnectionBuffer(ByteArray(40), 0, 40)
            for (offset in listOf(Int.MIN_VALUE, -1, 41, Int.MAX_VALUE)) {
                reject(io, ConnectionBuffer(good.bytes, offset, 1))
                reject(io, good, ConnectionBuffer(good.bytes, offset, 1))
            }
            for (length in listOf(Int.MIN_VALUE, -1, 0, 41, 65537, Int.MAX_VALUE)) {
                reject(io, ConnectionBuffer(good.bytes, 0, length))
                reject(io, good, ConnectionBuffer(good.bytes, 0, length))
            }
            val maximum = ConnectionBuffer(ByteArray(ConnectionWire.MAX_PAYLOAD), 0, ConnectionWire.MAX_PAYLOAD)
            repeat(128) { reject(io, maximum, ConnectionBuffer(ByteArray(168), 0, 41 + it)) }
            assertTrue(output.slices.isEmpty()); assertEquals(0, output.flushes)
            assertTrue(io.submit(transaction(1))); assertNull(completion(io).failure)
        } finally { retire(io) }
    }
}
