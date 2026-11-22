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

private fun localWriteAwait(condition: () -> Boolean) {
    val deadline = System.nanoTime() + TimeUnit.SECONDS.toNanos(3)
    while (!condition()) {
        check(System.nanoTime() < deadline) { "T1758 native fixture did not settle" }
        Thread.sleep(1)
    }
}

private class LocalFlushGate(val failure: IOException? = null) : OutputStream() {
    val bytes = ByteArrayOutputStream()
    val entered = CountDownLatch(1)
    val release = CountDownLatch(1)
    override fun write(value: Int) = bytes.write(value)
    override fun write(source: ByteArray, offset: Int, length: Int) = bytes.write(source, offset, length)
    override fun flush() {
        entered.countDown()
        check(release.await(3, TimeUnit.SECONDS)) { "T1758 fixture did not release flush" }
        failure?.let { throw it }
    }
}

private class LocalNativeWrite(failure: IOException? = null) : AutoCloseable {
    val rig = LocalPumpRig()
    val receipt = rig.incoming()
    val output = LocalFlushGate(failure)
    val consumer = LocalChannelConsumer(8, 10)
    // The byte-array streams have no native close effect; blocked flush must
    // still leave the original worker unretired after this closer returns.
    val io = StreamConnectionIo(ByteArrayInputStream(byteArrayOf()), output, Closeable {})
    fun blockedFlush() {
        consumer.advance(rig.endpoint, rig.lease, ConnectionWire.CONTROL, io, 0)
        localWriteAwait {
            if (output.entered.count == 0L) true else {
                io.poll()?.let { complete(it, false, 0) }
                consumer.advance(rig.endpoint, rig.lease, ConnectionWire.CONTROL, io, 0)
                output.entered.count == 0L
            }
        }
        assertArrayEquals(byteArrayOf(1, 2, 3), output.bytes.toByteArray())
        assertSame(receipt, rig.endpoint.borrowed); assertTrue(rig.endpoint.consumed.isEmpty())
        assertNull(io.poll())
    }
    fun complete(result: ConnectionIoCompletion, stopped: Boolean, now: Long) {
        consumer.complete(result, rig.endpoint, rig.lease, ConnectionWire.CONTROL, stopped, now)
    }
    fun completion(): ConnectionIoCompletion {
        var result: ConnectionIoCompletion? = null
        localWriteAwait { result = io.poll(); result != null }
        return checkNotNull(result)
    }
    override fun close() {
        output.release.countDown(); io.stop(); localWriteAwait(io::retired)
        assertNull(io.closeFailure())
    }
}

class ConnectionLocalTransactionTest {
    @Test fun t1758ActualFlushReturnIsRequiredBeforeRingCreditRelease() {
        LocalNativeWrite().use { fixture ->
            fixture.blockedFlush()
            fixture.output.release.countDown()
            fixture.complete(fixture.completion(), false, 9)
            assertSame(fixture.receipt, fixture.rig.endpoint.consumed.single())
            assertEquals(ConnectionDeadline.IDLE, fixture.consumer.nextDeadline())
        }
    }
    @Test fun t1758ExpiredBlockedFlushKeepsBorrowUntilOriginalNativeExit() {
        LocalNativeWrite().use { fixture ->
            fixture.blockedFlush()
            assertThrows(IllegalStateException::class.java) { fixture.consumer.tick(10) }
            fixture.io.stop(); assertFalse(fixture.io.retired())
            assertSame(fixture.receipt, fixture.rig.endpoint.borrowed)
            fixture.output.release.countDown()
            fixture.complete(fixture.completion(), true, 11)
            localWriteAwait(fixture.io::retired)
            assertTrue(fixture.rig.endpoint.consumed.isEmpty())
            assertSame(fixture.receipt, fixture.rig.endpoint.borrowed)
        }
    }
    @Test fun t1758NativeFlushFailureNeverReleasesTheWrittenPayload() {
        val failure = IOException("flush failed")
        LocalNativeWrite(failure).use { fixture ->
            fixture.blockedFlush(); fixture.output.release.countDown()
            val result = fixture.completion()
            assertSame(failure, result.failure)
            assertSame(failure, assertThrows(IOException::class.java) { fixture.complete(result, false, 1) })
            assertSame(fixture.receipt, fixture.rig.endpoint.borrowed)
            assertTrue(fixture.rig.endpoint.consumed.isEmpty())
        }
    }
}
