package com.blent.connection

import com.blent.JvmAllocations
import org.junit.Assert.*
import org.junit.Test

/** T987: reuse scratch only after original completion and pending-byte acceptance. */
class ConnectionProducerAllocationTest {
    @Test fun t987PartialReadsReuseScratchOnlyAfterCopiedTransportBytesAreAccepted() {
        for (length in 1..7) {
            val rig = LocalPumpRig(); rig.endpoint.accepting = false; rig.pump.advance(0)
            val original = rig.io.read()
            rig.pump.advance(1); assertSame(original, rig.io.read())
            val payload = ByteArray(length) { it.toByte() }
            rig.read(payload, 2)
            val admitted = rig.io.requests.size
            rig.pump.advance(3)
            assertEquals(admitted, rig.io.requests.size)
            assertTrue(rig.endpoint.sent.isEmpty())
            rig.endpoint.accepting = true; rig.pump.advance(4)
            assertNotSame(original, rig.io.read())
            assertSame("T987 reuse completed scratch after acceptance", original.loan.bytes, rig.io.read().loan.bytes)
            assertNotSame(original.loan.bytes, rig.endpoint.sent.single())
            rig.io.read().loan.bytes.fill(99)
            assertArrayEquals(payload, rig.endpoint.sent.single())
            assertNull(rig.pump.failure())
        }
    }

    @Test fun t987ZeroCompletionsReuseScratchWithoutPublishingBytesOrRepeatingRequests() {
        val rig = LocalPumpRig(); rig.pump.advance(0)
        val buffer = rig.io.read().loan.bytes
        repeat(8) { turn ->
            val original = rig.io.read()
            rig.io.complete(original, 0); rig.pump.advance(turn + 1L)
            assertNotSame(original, rig.io.read())
            assertSame("T987 empty completion keeps original allocation", buffer, rig.io.read().loan.bytes)
        }
        assertTrue(rig.endpoint.sent.isEmpty()); assertNull(rig.pump.failure())
    }

    @Test fun t987TransferredFullBlockIsNeverReusedAfterPartialScratch() {
        val rig = LocalPumpRig(); rig.pump.advance(0)
        rig.read(byteArrayOf(1, 2), 1)
        val scratch = rig.io.read().loan.bytes
        val full = ByteArray(8) { it.toByte() }
        rig.read(full, 2)
        assertSame(scratch, rig.endpoint.sent[1])
        assertNotSame(scratch, rig.io.read().loan.bytes)
        rig.io.read().loan.bytes.fill(99)
        assertArrayEquals(full, rig.endpoint.sent[1])
        assertArrayEquals(byteArrayOf(1, 2), rig.endpoint.sent[0])
    }

    @Test fun t987ReusedArrayCannotMakeAFormerRequestCompletionCurrent() {
        val rig = LocalPumpRig(); rig.pump.advance(0)
        val original = rig.io.read()
        rig.read(byteArrayOf(3), 1)
        rig.io.complete(original, 1); rig.pump.advance(2)
        assertNotNull(rig.pump.failure()); assertTrue(rig.io.stopping)
        assertEquals(1, rig.endpoint.sent.size)
    }

    @Test fun t987StoppedAndStalledOwnersCannotRebindScratchOrReleaseAnotherPump() {
        val rig = LocalPumpRig(); val sibling = LocalPumpRig()
        rig.pump.advance(0); sibling.pump.advance(0)
        val original = rig.io.read(); val admitted = rig.io.requests.size
        rig.pump.stop(1)
        rig.io.complete(original, 2); rig.pump.advance(2)
        rig.pump.advance(6)
        assertEquals(admitted, rig.io.requests.size)
        assertTrue(rig.pump.quarantined()); assertFalse(rig.pump.retired())
        sibling.read(byteArrayOf(7), 1)
        assertFalse(sibling.io.stopping); assertArrayEquals(byteArrayOf(7), sibling.endpoint.sent.single())
        assertNotSame(original.loan.bytes, sibling.io.read().loan.bytes)
        rig.io.gone = true; rig.pump.advance(7); assertTrue(rig.pump.retired())
    }

    @Test fun t987InvalidCompletionAfterReuseNeverPublishesScratchContents() {
        for (count in listOf(Int.MIN_VALUE, -1, 9, Int.MAX_VALUE)) {
            val rig = LocalPumpRig(); rig.pump.advance(0)
            rig.read(byteArrayOf(4, 5), 1)
            val current = rig.io.read()
            current.loan.bytes.fill(99)
            rig.io.complete(current, count); rig.pump.advance(2)
            assertNotNull(rig.pump.failure()); assertTrue(rig.io.stopping)
            assertEquals(1, rig.endpoint.sent.size)
            assertArrayEquals(byteArrayOf(4, 5), rig.endpoint.sent.single())
        }
    }

    @Test fun t987BoundedReadExtentsPreserveContentAndStorageOwnership() {
        for (block in listOf(1, 2, 8, ConnectionWire.MAX_PAYLOAD)) {
            for (count in listOf(0, 1, block - 1, block).distinct()) {
                val rig = LocalPumpRig(block); rig.pump.advance(0)
                val original = rig.io.read().loan.bytes
                val payload = ByteArray(count) { (it % 251).toByte() }
                rig.read(payload, 1)
                if (count < block) assertSame(original, rig.io.read().loan.bytes)
                else assertNotSame(original, rig.io.read().loan.bytes)
                rig.io.read().loan.bytes.fill(99)
                if (count > 0) assertArrayEquals(payload, rig.endpoint.sent.single())
                else assertTrue(rig.endpoint.sent.isEmpty())
                assertNull(rig.pump.failure())
            }
        }
    }

    private class Sink : ConnectionEndpoints by RunnerSession() {
        var retained: ByteArray? = null
        override fun offer(lease: ConnectionResourceLease, channel: Int, bytes: ByteArray): Boolean {
            retained = bytes
            return true
        }
    }
    private class Io : ConnectionIo {
        lateinit var read: ConnectionIoRequest.Read
        override fun submit(request: ConnectionIoRequest): Boolean { read = request as ConnectionIoRequest.Read; return true }
        override fun poll(): ConnectionIoCompletion? = null
        override fun stop() {}
        override fun retired() = false
        override fun closeFailure(): Exception? = null
    }
    private class Sample(val label: String, val channel: Int, val count: Int?)
    private class Measurement(private val sample: Sample) {
        private val producer = LocalChannelProducer(ConnectionWire.MAX_PAYLOAD, 1000)
        private val io = Io()
        private val sink = Sink()
        private val lease = audioLease(ConnectionChannels.resource(sample.channel))
        init { producer.advance(sink, lease, sample.channel, io, 0) }
        fun step() {
            val count = sample.count
            if (count != null) {
                io.read.loan.bytes.fill(7, 0, count)
                producer.complete(ConnectionIoCompletion(io.read, count), false, 0)
            }
            producer.advance(sink, lease, sample.channel, io, 0)
        }
    }
    private fun measure(sample: Sample) {
        val rig = Measurement(sample)
        repeat(64) { rig.step() }
        val allocations = JvmAllocations()
        repeat(3) { index ->
            val before = allocations.read()
            val started = System.nanoTime()
            repeat(256) { rig.step() }
            val elapsed = System.nanoTime() - started
            val allocated = allocations.read() - before
            println("T987 case=${sample.label} sample=$index reads=256 bytes=$allocated nanos=$elapsed")
        }
    }
    @Test fun t987BoundedIdleEmptyPcmAndDisplayAllocationMeasurements() {
        val cases = listOf(Sample("idle", ConnectionWire.DISPLAY, null), Sample("empty", ConnectionWire.DISPLAY, 0),
            Sample("pcm-partial", ConnectionWire.MICROPHONE, 1920), Sample("pcm-full", ConnectionWire.MICROPHONE, 65536),
            Sample("display-partial", ConnectionWire.DISPLAY, 32768), Sample("display-full", ConnectionWire.DISPLAY, 65536))
        cases.forEach(::measure)
    }
}
