package com.blent.connection

import java.io.IOException
import java.util.ArrayDeque
import org.junit.Assert.*
import org.junit.Test

internal class LocalEndpoints : ConnectionEndpoints by RunnerSession() {
    var active = true
    var accepting = true
    val sent = ArrayList<ByteArray>()
    val receive = ArrayDeque<ConnectionChannelRead>()
    val consumed = ArrayList<ConnectionChannelRead>()
    var borrowed: ConnectionChannelRead? = null
    override fun current(lease: ConnectionResourceLease) = active
    override fun offer(lease: ConnectionResourceLease, channel: Int, bytes: ByteArray): Boolean {
        if (!accepting) return false
        sent.add(bytes)
        return true
    }
    override fun read(lease: ConnectionResourceLease, channel: Int, maxBytes: Int): ConnectionChannelRead? {
        check(borrowed == null)
        return receive.pollFirst()?.also { borrowed = it }
    }
    override fun consumed(lease: ConnectionResourceLease, channel: Int, receipt: ConnectionChannelRead) {
        check(borrowed === receipt)
        consumed.add(receipt); borrowed = null
    }
    override fun nativeRetired(lease: ConnectionResourceLease) { error("T905 whole-resource retirement belongs to outer owner") }
}
internal class LocalPumpRig(block: Int = 8) {
    val endpoint = LocalEndpoints()
    val io = RunnerIo()
    val lease = ConnectionResourceLease(ConnectionTicket(1, ConnectionPeer.parse("ab".repeat(32))),
        ConnectionManagement.Key(ConnectionManagement.Resource.DISPLAY, 1, 1), 1)
    val pump = ConnectionLocalPump(endpoint, lease, ConnectionWire.CONTROL, io, block, ConnectionRunLimits(10, 5))
    fun incoming(bytes: ByteArray = byteArrayOf(91, 1, 2, 3, 92), offset: Int = 1, length: Int = 3): ConnectionChannelRead =
        ConnectionChannelRead(bytes, offset, length).also { endpoint.receive.add(it) }
    fun read(bytes: ByteArray, now: Long = 1) {
        val read = io.read()
        bytes.copyInto(read.loan.bytes)
        io.complete(read, bytes.size); pump.advance(now)
    }
}

class ConnectionLocalPumpTest {
    @Test fun t905DirectRingLoanWaitsForAllWritesAndFlushWhileReaderStaysIdle() {
        val rig = LocalPumpRig(); val receipt = rig.incoming()
        rig.pump.advance(0)
        val read = rig.io.read(); val first = rig.io.write()
        assertSame(receipt.bytes, first.loan.bytes); assertEquals(1, first.loan.offset)
        rig.io.complete(first, 1); rig.pump.advance(1)
        val rest = rig.io.write()
        assertSame(receipt.bytes, rest.loan.bytes); assertEquals(2, rest.loan.offset); assertEquals(2, rest.loan.length)
        rig.io.complete(rest, 2); rig.pump.advance(2)
        assertTrue(rig.endpoint.consumed.isEmpty())
        rig.io.complete(rig.io.flush(), 0); rig.pump.advance(3)
        assertSame(receipt, rig.endpoint.consumed.single()); assertSame(read, rig.io.read())
        assertNull(rig.pump.failure()); assertFalse(rig.pump.retired()); assertFalse(rig.pump.quarantined())
    }
    @Test fun t905ProducerTransfersFullBlockAndBoundsPartialBlockWithoutReusingIt() {
        val rig = LocalPumpRig(); rig.pump.advance(0)
        val buffer = rig.io.read().loan.bytes
        rig.read(ByteArray(8) { it.toByte() })
        assertSame(buffer, rig.endpoint.sent.single()); assertNotSame(buffer, rig.io.read().loan.bytes)
        rig.read(byteArrayOf(4, 5, 6), 2)
        assertArrayEquals(byteArrayOf(4, 5, 6), rig.endpoint.sent[1]); assertEquals(3, rig.endpoint.sent[1].size)
        rig.io.complete(rig.io.read(), 0); rig.pump.advance(3)
        assertEquals(2, rig.endpoint.sent.size); assertNull(rig.pump.failure())
    }
    @Test fun t905BackpressureRetainsOneBlockAndDoesNotResetAbsoluteDeadline() {
        val rig = LocalPumpRig(); rig.endpoint.accepting = false; rig.pump.advance(0)
        rig.read(byteArrayOf(1, 2)); val requests = rig.io.requests.size
        for (now in 2L..10L) rig.pump.advance(now)
        assertEquals(requests, rig.io.requests.size); assertFalse(rig.io.stopping)
        rig.pump.advance(11)
        assertTrue(rig.io.stopping); assertNotNull(rig.pump.failure()); assertTrue(rig.endpoint.sent.isEmpty())
        assertFalse(rig.pump.quarantined()); rig.pump.advance(16); assertTrue(rig.pump.quarantined())
        rig.io.gone = true; rig.pump.advance(17)
        assertTrue(rig.pump.retired()); assertFalse(rig.pump.quarantined())
    }
    @Test fun t905ReleasedBackpressureResumesBothDirectionsWithoutDroppingBytes() {
        val rig = LocalPumpRig(); rig.endpoint.accepting = false; rig.incoming(); rig.pump.advance(0)
        val write = rig.io.write(); rig.read(byteArrayOf(5, 6)); rig.pump.advance(2)
        assertSame(write, rig.io.write())
        rig.endpoint.accepting = true; rig.pump.advance(3)
        assertArrayEquals(byteArrayOf(5, 6), rig.endpoint.sent.single())
        assertSame(write, rig.io.write()); assertNull(rig.pump.failure())
    }
    @Test fun t905ExpiredConsumerBudgetRejectsProducerBytesBeforeAnyNewTransfer() {
        val rig = LocalPumpRig(); rig.incoming(); rig.pump.advance(0)
        val read = rig.io.read(); read.loan.bytes[0] = 7
        rig.io.complete(read, 1)
        rig.pump.advance(10)
        assertNotNull(rig.pump.failure())
        assertTrue("T905 check all active deadlines before forwarding fresh local bytes", rig.endpoint.sent.isEmpty())
        assertTrue(rig.endpoint.consumed.isEmpty())
    }

    @Test fun t905IdleReadHasNoProgressDeadlineButBlockedFlushDoes() {
        val rig = LocalPumpRig(); rig.pump.advance(0); val read = rig.io.read()
        rig.pump.advance(1000); assertNull(rig.pump.failure()); assertSame(read, rig.io.read())
        rig.incoming(); rig.pump.advance(1001)
        rig.io.complete(rig.io.write(), 3); rig.pump.advance(1009)
        rig.io.complete(rig.io.flush(), 0); rig.pump.advance(1011)
        assertNotNull(rig.pump.failure()); assertTrue(rig.endpoint.consumed.isEmpty())
    }
    @Test fun t905RevocationSuppressesLateCompletionWithoutReleasingBorrowedRing() {
        val rig = LocalPumpRig(); val receipt = rig.incoming(); rig.pump.advance(0)
        val read = rig.io.read(); val write = rig.io.write()
        rig.endpoint.active = false
        rig.io.complete(read, 2); rig.io.complete(write, 3); rig.pump.advance(1)
        assertTrue(rig.io.stopping); assertTrue(rig.endpoint.sent.isEmpty()); assertTrue(rig.endpoint.consumed.isEmpty())
        assertSame(receipt, rig.endpoint.borrowed); assertFalse(rig.pump.retired()); assertNull(rig.pump.failure())
        rig.pump.stop(2); rig.io.gone = true; rig.pump.advance(3); assertTrue(rig.pump.retired())
        rig.io.complete(read, -1); rig.pump.advance(4); assertNull(rig.pump.failure())
    }
    @Test fun t905MalformedOrForeignCompletionsFailClosed() {
        for (count in listOf(Int.MIN_VALUE, -1, 9, Int.MAX_VALUE)) {
            val rig = LocalPumpRig(); rig.pump.advance(0)
            rig.io.complete(rig.io.read(), count); rig.pump.advance(1)
            assertTrue(rig.io.stopping); assertNotNull(rig.pump.failure()); assertTrue(rig.endpoint.sent.isEmpty())
        }
        val rig = LocalPumpRig(); rig.pump.advance(0)
        val foreign = rig.io.read().copy()
        rig.io.complete(foreign, 1); rig.pump.advance(1)
        assertNotNull(rig.pump.failure()); assertTrue(rig.endpoint.sent.isEmpty())
    }
    @Test fun t905InvalidWriterProgressAndForeignFlushNeverReturnCredit() {
        for (count in listOf(Int.MIN_VALUE, -1, 0, 4, Int.MAX_VALUE)) {
            val rig = LocalPumpRig(); rig.incoming(); rig.pump.advance(0)
            rig.io.complete(rig.io.write(), count); rig.pump.advance(1)
            assertNotNull(rig.pump.failure()); assertTrue(rig.endpoint.consumed.isEmpty())
        }
        val rig = LocalPumpRig(); rig.incoming(); rig.pump.advance(0)
        rig.io.complete(ConnectionIoRequest.Flush(ConnectionFlush()), 0); rig.pump.advance(1)
        assertNotNull(rig.pump.failure()); assertTrue(rig.endpoint.consumed.isEmpty())
    }
    @Test fun t905FlushCountAndNativeErrorsRetainOriginalFailure() {
        val rig = LocalPumpRig(); rig.incoming(); rig.pump.advance(0)
        rig.io.complete(rig.io.write(), 3); rig.pump.advance(1)
        rig.io.complete(rig.io.flush(), 1); rig.pump.advance(2)
        assertNotNull(rig.pump.failure()); assertTrue(rig.endpoint.consumed.isEmpty())
        val failure = IOException("read failed"); val other = LocalPumpRig(); other.pump.advance(0)
        other.io.complete(other.io.read(), 0, failure); other.pump.advance(1)
        assertSame(failure, other.pump.failure())
        other.io.closeError = IOException("close also failed"); other.io.gone = true; other.pump.advance(2)
        assertSame(failure, other.pump.failure()); assertFalse(other.pump.retired()); assertTrue(other.pump.quarantined())
    }
    @Test fun t905WriteFailureAndRejectedAdmissionStopOnlyTheirOwnChannel() {
        val rig = LocalPumpRig(); val sibling = LocalPumpRig(); rig.incoming(); rig.pump.advance(0); sibling.pump.advance(0)
        val failure = IOException("write failed")
        rig.io.complete(rig.io.write(), 0, failure); rig.pump.advance(1)
        sibling.read(byteArrayOf(7), 1)
        assertSame(failure, rig.pump.failure()); assertFalse(sibling.io.stopping)
        assertArrayEquals(byteArrayOf(7), sibling.endpoint.sent.single())
        val rejected = LocalPumpRig(); rejected.io.accepting = false; rejected.pump.advance(0)
        assertNotNull(rejected.pump.failure()); assertTrue(rejected.io.stopping)
    }
    @Test fun t905DuplicateCompletionsCannotOfferSameBytesTwice() {
        val rig = LocalPumpRig(); rig.pump.advance(0); val read = rig.io.read()
        rig.io.complete(read, 1); rig.pump.advance(1)
        rig.io.complete(read, 1); rig.pump.advance(2)
        assertEquals(1, rig.endpoint.sent.size); assertNotNull(rig.pump.failure())
    }
    @Test fun t905InvalidConsumerSlicesNeverReachNativeWriter() {
        for ((offset, length) in listOf(-1 to 1, 6 to 1, 0 to 0, 0 to 9, 4 to 2, Int.MAX_VALUE to Int.MAX_VALUE)) {
            val rig = LocalPumpRig(); rig.incoming(ByteArray(5), offset, length); rig.pump.advance(0)
            assertNotNull(rig.pump.failure())
            assertTrue(rig.io.requests.none { it is ConnectionIoRequest.Write })
            assertTrue(rig.endpoint.consumed.isEmpty())
        }
    }
    @Test fun t905LimitsLeaseAndClockBoundariesRejectWithoutNativeWork() {
        for (block in listOf(Int.MIN_VALUE, -1, 0, 65537, Int.MAX_VALUE)) {
            assertThrows(IllegalArgumentException::class.java) { LocalPumpRig(block) }
            assertThrows(IllegalArgumentException::class.java) { LocalChannelConsumer(block, 10) }
        }
        val rig = LocalPumpRig()
        for (channel in listOf(Int.MIN_VALUE, 0, 3, 6, Int.MAX_VALUE)) {
            assertThrows(IllegalArgumentException::class.java) { ConnectionLocalPump(rig.endpoint, rig.lease, channel, rig.io) }
        }
        for (now in listOf(Long.MIN_VALUE, -1, Long.MAX_VALUE)) {
            assertThrows(IllegalArgumentException::class.java) { rig.pump.advance(now) }
        }
        assertTrue(rig.io.requests.isEmpty()); rig.pump.advance(3)
        assertThrows(IllegalArgumentException::class.java) { rig.pump.stop(2) }
        assertFalse(rig.io.stopping)
    }
    @Test fun t905InvalidLeaseCountersRejectAndMaximumCountersRemainRepresentable() {
        val rig = LocalPumpRig()
        for (value in listOf(Long.MIN_VALUE, -1, 0)) {
            for (lease in listOf(rig.lease.copy(owner = value), rig.lease.copy(connection = rig.lease.connection.copy(epoch = value)),
                rig.lease.copy(key = rig.lease.key.copy(request = value)), rig.lease.copy(key = rig.lease.key.copy(generation = value)))) {
                assertThrows(IllegalArgumentException::class.java) { ConnectionLocalPump(rig.endpoint, lease, 1, rig.io) }
            }
        }
        val maximum = rig.lease.copy(owner = Long.MAX_VALUE, key = rig.lease.key.copy(generation = Long.MAX_VALUE))
        ConnectionLocalPump(rig.endpoint, maximum, 1, rig.io).stop(0)
        assertTrue(rig.io.stopping)
    }
    @Test fun t905CancelledFlushAndRejectedWriteNeverAcknowledgeBorrow() {
        val rig = LocalPumpRig(); rig.incoming(); rig.pump.advance(0)
        rig.io.complete(rig.io.write(), 3); rig.pump.advance(1)
        rig.pump.stop(2); rig.io.complete(rig.io.flush(), 0); rig.pump.advance(3)
        assertTrue(rig.endpoint.consumed.isEmpty()); assertFalse(rig.pump.retired())
        val rejected = LocalPumpRig(); rejected.incoming()
        val io = object : ConnectionIo by rejected.io {
            override fun submit(request: ConnectionIoRequest) = if (request is ConnectionIoRequest.Write) false else rejected.io.submit(request)
        }
        val pump = ConnectionLocalPump(rejected.endpoint, rejected.lease, 1, io, 8)
        pump.advance(0)
        assertNotNull(pump.failure()); assertTrue(rejected.endpoint.consumed.isEmpty())
    }
    @Test fun t905ThrowingStopAndCloseFailureRemainQuarantinedWithoutReplacement() {
        val rig = LocalPumpRig(); val failure = IOException("stop failed")
        val io = object : ConnectionIo by rig.io { override fun stop() { throw failure } }
        val pump = ConnectionLocalPump(rig.endpoint, rig.lease, 1, io, 8, ConnectionRunLimits(10, 5))
        pump.stop(0); assertSame(failure, pump.failure()); assertFalse(pump.retired())
        pump.advance(5); assertTrue(pump.quarantined())
        val close = LocalPumpRig(); close.pump.stop(0); close.io.closeError = failure; close.io.gone = true; close.pump.advance(1)
        assertSame(failure, close.pump.failure()); assertFalse(close.pump.retired()); assertTrue(close.pump.quarantined())
    }
    @Test fun t905NativeLoopbackDuplexTransfersAndActuallyRetiresItsIoWorkers() {
        val pair = SocketBridgeSockets.open()
        pair.client.soTimeout = 1000
        val endpoint = LocalEndpoints(); val rig = LocalPumpRig()
        val io = StreamConnectionIo(pair.relay.getInputStream(), pair.relay.getOutputStream(), pair)
        val pump = ConnectionLocalPump(endpoint, rig.lease, 1, io, 8)
        val origin = System.nanoTime()
        fun now() = (System.nanoTime() - origin) / 1_000_000
        try {
            endpoint.receive.add(ConnectionChannelRead(byteArrayOf(1, 2, 3), 0, 3))
            pair.client.getOutputStream().write(byteArrayOf(4, 5, 6))
            while (endpoint.sent.isEmpty() || endpoint.consumed.isEmpty()) {
                assertTrue("T905 loopback stalled", now() < 3000)
                pump.advance(now()); Thread.sleep(1)
            }
            assertArrayEquals(byteArrayOf(4, 5, 6), endpoint.sent.flatMap { it.toList() }.toByteArray())
            val bytes = ByteArray(3)
            java.io.DataInputStream(pair.client.getInputStream()).readFully(bytes)
            assertArrayEquals(byteArrayOf(1, 2, 3), bytes)
        } finally {
            pump.stop(now())
            while (!pump.retired()) {
                assertTrue("T905 native workers failed to retire", now() < 5000)
                pump.advance(now()); Thread.sleep(1)
            }
        }
        assertNull(pump.failure())
    }

    @Test fun t905SeededPartialTransfersPreserveEveryByteAndExactCredit() {
        val random = java.util.Random(905)
        repeat(512) {
            val rig = LocalPumpRig()
            val expected = ByteArray(1 + random.nextInt(8)) { random.nextInt().toByte() }
            val receipt = rig.incoming(expected.copyOf(), 0, expected.size)
            rig.pump.advance(0); rig.read(expected)
            var written = 0
            while (written < expected.size) {
                val request = rig.io.write()
                val count = 1 + random.nextInt(request.loan.length)
                assertArrayEquals(expected.copyOfRange(written, written + count), request.loan.bytes.copyOfRange(request.loan.offset, request.loan.offset + count))
                rig.io.complete(request, count); rig.pump.advance(2); written += count
            }
            assertTrue(rig.endpoint.consumed.isEmpty()); rig.io.complete(rig.io.flush(), 0); rig.pump.advance(3)
            assertSame(receipt, rig.endpoint.consumed.single()); assertArrayEquals(expected, rig.endpoint.sent.single())
            assertNull(rig.pump.failure())
        }
    }
}
