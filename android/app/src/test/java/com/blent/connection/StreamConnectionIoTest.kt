package com.blent.connection

import java.io.ByteArrayInputStream
import java.io.ByteArrayOutputStream
import java.io.Closeable
import java.io.IOException
import java.io.InputStream
import java.io.OutputStream
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicInteger
import org.junit.Assert.*
import org.junit.Test

class StreamConnectionIoTest {
    private fun await(condition: () -> Boolean) {
        val deadline = System.nanoTime() + TimeUnit.SECONDS.toNanos(3)
        while (!condition()) {
            check(System.nanoTime() < deadline) { "T883 native worker deadline" }
            Thread.sleep(1)
        }
    }
    private fun completion(io: ConnectionIo): ConnectionIoCompletion {
        var result: ConnectionIoCompletion? = null
        await { result = io.poll(); result != null }
        return checkNotNull(result)
    }
    private fun read(bytes: ByteArray = ByteArray(8), offset: Int = 0, length: Int = bytes.size) =
        ConnectionIoRequest.Read(ConnectionBuffer(bytes, offset, length))
    private fun write(bytes: ByteArray = byteArrayOf(1, 2), offset: Int = 0, length: Int = bytes.size) =
        ConnectionIoRequest.Write(ConnectionBuffer(bytes, offset, length))
    private fun retire(io: ConnectionIo) { io.stop(); await(io::retired) }
    private fun latch(latch: CountDownLatch) { check(latch.await(3, TimeUnit.SECONDS)) { "T883 worker never entered" } }

    @Test fun t883StreamsPreservePartialReadOffsetsEofAndWriteFlush() {
        val output = ByteArrayOutputStream()
        val io = StreamConnectionIo(ByteArrayInputStream(byteArrayOf(7, 8)), output)
        try {
            assertNull(io.poll()); assertFalse(io.retired())
            val request = read(ByteArray(5) { 99 }, 1, 3)
            assertTrue(io.submit(request))
            val result = completion(io); assertSame(request, result.request); assertEquals(2, result.count)
            assertArrayEquals(byteArrayOf(99, 7, 8, 99, 99), request.loan.bytes)
            assertTrue(io.submit(read())); assertEquals(-1, completion(io).count)
            val sending = write(byteArrayOf(3, 4, 5, 6), 1, 2)
            assertTrue(io.submit(sending)); assertEquals(2, completion(io).count)
            val flush = ConnectionIoRequest.Flush(ConnectionFlush())
            assertTrue(io.submit(flush)); assertSame(flush, completion(io).request)
            assertArrayEquals(byteArrayOf(4, 5), output.toByteArray()); assertNull(io.closeFailure())
        } finally { retire(io) }
        assertFalse(io.submit(read())); assertNull(io.poll())
    }
    private class GateInput : InputStream() {
        val entered = CountDownLatch(1)
        val release = CountDownLatch(1)
        override fun read(): Int { entered.countDown(); check(release.await(3, TimeUnit.SECONDS)); return -1 }
        override fun read(bytes: ByteArray, offset: Int, length: Int): Int = read()
        override fun close() { release.countDown() }
    }
    @Test fun t883StalledReadLeavesWriteAndCloseIndependent() {
        val input = GateInput()
        val output = ByteArrayOutputStream()
        val io = StreamConnectionIo(input, output)
        try {
            val request = read(); assertTrue(io.submit(request)); latch(input.entered)
            assertFalse(io.submit(read()))
            assertTrue(io.submit(write())); assertEquals(2, completion(io).count)
            assertEquals(2, output.size()); assertFalse(io.retired())
            io.stop(); assertFalse(io.submit(write())); await(io::retired)
            val done = completion(io); assertSame(request, done.request); assertEquals(-1, done.count)
        } finally { input.release.countDown(); retire(io) }
    }
    @Test fun t883CompletionRetainsSingleFlightSlotUntilConsumed() {
        val output = CountDownLatch(1)
        val io = StreamConnectionIo(ByteArrayInputStream(byteArrayOf(4)), object : OutputStream() {
            override fun write(value: Int) { output.countDown() }
        })
        try {
            val first = write(byteArrayOf(9)); assertTrue(io.submit(first)); latch(output)
            assertFalse(io.submit(write())); assertFalse(io.submit(ConnectionIoRequest.Flush(ConnectionFlush())))
            assertSame(first, completion(io).request)
            assertTrue(io.submit(ConnectionIoRequest.Flush(ConnectionFlush()))); completion(io)
        } finally { retire(io) }
    }
    @Test fun t883BothCompletedDirectionsArePolledFairly() {
        val io = StreamConnectionIo(ByteArrayInputStream(ByteArray(16) { 1 }), ByteArrayOutputStream())
        try {
            repeat(8) {
                val reading = read(length = 1); val writing = write()
                assertTrue(io.submit(reading)); assertTrue(io.submit(writing))
                val results = listOf(completion(io), completion(io))
                assertEquals(setOf(reading, writing), results.map { it.request }.toSet())
            }
        } finally { retire(io) }
    }
    @Test fun t883NativeReadWriteAndFlushFailuresPreserveExactReceipt() {
        val io = StreamConnectionIo(object : InputStream() {
            override fun read(): Int = throw IOException("read")
        }, object : OutputStream() {
            override fun write(value: Int) { throw IOException("write") }
            override fun flush() { throw IOException("flush") }
        })
        try {
            for (request in listOf(read(), write(), ConnectionIoRequest.Flush(ConnectionFlush()))) {
                assertTrue(io.submit(request))
                val result = completion(io)
                assertSame(request, result.request); assertTrue(result.failure is IOException); assertEquals(0, result.count)
            }
        } finally { retire(io) }
    }
    @Test fun t883InvalidLoansRejectBeforeAdmissionAndLeaveValidRequestUsable() {
        val io = StreamConnectionIo(ByteArrayInputStream(byteArrayOf(1)), ByteArrayOutputStream())
        try {
            for (offset in listOf(Int.MIN_VALUE, -1, 9, Int.MAX_VALUE)) {
                assertThrows(IllegalArgumentException::class.java) { io.submit(read(offset = offset)) }
            }
            for (length in listOf(Int.MIN_VALUE, -1, 0, 9, 65537, Int.MAX_VALUE)) {
                assertThrows(IllegalArgumentException::class.java) { io.submit(read(length = length)) }
            }
            assertTrue(io.submit(read())); assertEquals(1, completion(io).count)
        } finally { retire(io) }
    }
    @Test fun t883BlockedCloseNeverClaimsNativeRetirement() {
        val entered = CountDownLatch(1); val release = CountDownLatch(1)
        val closes = AtomicInteger()
        val io = StreamConnectionIo(ByteArrayInputStream(byteArrayOf()), ByteArrayOutputStream(), Closeable {
            closes.incrementAndGet(); entered.countDown(); check(release.await(3, TimeUnit.SECONDS))
        })
        try {
            io.stop(); io.stop(); latch(entered)
            assertFalse(io.retired()); assertNull(io.closeFailure()); assertFalse(io.submit(read()))
        } finally { release.countDown(); retire(io) }
        assertEquals(1, closes.get())
    }
    @Test fun t883CloseFailureIsReportedAfterActualNativeExit() {
        val io = StreamConnectionIo(ByteArrayInputStream(byteArrayOf()), ByteArrayOutputStream(), Closeable { throw IOException("close") })
        retire(io)
        assertEquals("close", io.closeFailure()?.message)
    }
    @Test fun t883PartialStartupFailureRetainsAndClosesExistingNativeOwnership() {
        val closed = AtomicInteger()
        val io = StreamConnectionIo(ByteArrayInputStream(byteArrayOf()), ByteArrayOutputStream(), Closeable { closed.incrementAndGet() }) { name, work ->
            if (name.endsWith("write")) throw IOException("start")
            Thread(work, name)
        }
        await(io::retired)
        assertEquals(1, closed.get()); assertEquals("start", io.closeFailure()?.message)
        assertFalse(io.submit(read()))
    }
    @Test fun t883MissingCloseWorkerLeavesExplicitQuarantinedOwner() {
        val closed = AtomicInteger()
        val io = StreamConnectionIo(ByteArrayInputStream(byteArrayOf()), ByteArrayOutputStream(), Closeable { closed.incrementAndGet() }) { name, work ->
            object : Thread(work, name) { override fun start() { throw IOException("no native closer") } }
        }
        io.stop(); assertFalse(io.retired()); assertEquals(0, closed.get())
        assertEquals("no native closer", io.closeFailure()?.message); assertFalse(io.submit(read()))
    }
    @Test fun t883StalledWriteLeavesReadAndCloseIndependent() {
        val entered = CountDownLatch(1); val release = CountDownLatch(1)
        val output = object : OutputStream() {
            override fun write(value: Int) { entered.countDown(); check(release.await(3, TimeUnit.SECONDS)) }
            override fun close() { release.countDown() }
        }
        val io = StreamConnectionIo(ByteArrayInputStream(byteArrayOf(9)), output)
        try {
            val sending = write(); assertTrue(io.submit(sending)); latch(entered)
            val reading = read(); assertTrue(io.submit(reading))
            assertSame(reading, completion(io).request)
            assertFalse(io.submit(ConnectionIoRequest.Flush(ConnectionFlush())))
            io.stop(); await(io::retired)
            assertSame(sending, completion(io).request)
        } finally { release.countDown(); retire(io) }
    }
    @Test fun t883CompletionWakeCanPollOutsideLocksAndCoversCloseProgress() {
        val wakes = AtomicInteger()
        val notified = CountDownLatch(1)
        val io = StreamConnectionIo(ByteArrayInputStream(byteArrayOf(1)), ByteArrayOutputStream(), ready = {
            wakes.incrementAndGet(); notified.countDown()
        })
        try {
            assertTrue(io.submit(read())); latch(notified)
            assertNotNull(completion(io)); assertTrue(wakes.get() >= 1)
        } finally { retire(io) }
        assertTrue(wakes.get() >= 2)
    }
    @Test fun t883ThrowingWakeCallbackIsContainedAndRevokesAdmission() {
        val io = StreamConnectionIo(ByteArrayInputStream(byteArrayOf(1)), ByteArrayOutputStream(), ready = {
            throw IOException("wakeup rejected")
        })
        assertTrue(io.submit(read())); await(io::retired)
        assertEquals("wakeup rejected", io.closeFailure()?.message)
        assertFalse(io.submit(write())); assertNotNull(completion(io))
    }

    @Test fun t883InterruptedIdleWorkerFailsClosedAndRetiresOwnership() {
        val reading = java.util.concurrent.atomic.AtomicReference<Thread>()
        val io = StreamConnectionIo(ByteArrayInputStream(byteArrayOf()), ByteArrayOutputStream()) { name, work ->
            Thread(work, name).also { thread ->
                thread.uncaughtExceptionHandler = Thread.UncaughtExceptionHandler { _, _ -> }
                if (name.endsWith("read")) reading.set(thread)
            }
        }
        try {
            reading.get().interrupt(); await { !reading.get().isAlive }
            assertNotNull("Idle worker loss must remain observable", io.closeFailure())
            await(io::retired); assertFalse(io.submit(read()))
        } finally { retire(io) }
    }

}
