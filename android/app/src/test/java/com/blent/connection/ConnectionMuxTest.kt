package com.blent.connection

import org.junit.Assert.*
import org.junit.Test

class ConnectionMuxTest {
    private fun mux() = ConnectionMux(7, 9, 64, 16, 4)
    private fun rejected(work: () -> Unit) { assertThrows(Exception::class.java) { work() } }
    private fun frame(owner: ConnectionMux): Pair<ConnectionWire.Header, ByteArray>? {
        val value = owner.next() ?: return null
        val result = value.header to value.bytes.copyOfRange(value.offset, value.offset + value.length)
        owner.finish(true)
        return result
    }
    private fun grant(left: ConnectionMux, right: ConnectionMux) {
        for (channel in 1..ConnectionWire.CHANNELS) {
            val (header, bytes) = checkNotNull(frame(left))
            assertEquals(0, bytes.size); assertEquals(channel, header.channel)
            assertEquals(ConnectionWire.CREDIT, header.kind); assertFalse(right.receive(header))
            assertFalse(left.receive(checkNotNull(frame(right)).first))
        }
        assertNull(frame(left)); assertNull(frame(right))
    }
    @Test fun t846MetadataBootstrapsAndCoalescesCredit() {
        val left = mux(); val right = mux(); grant(left, right)
        val bytes = ByteArray(8) { 42 }; assertTrue(left.offer(2, bytes))
        for (offset in listOf(0, 4)) {
            val selected = checkNotNull(left.next())
            assertSame(bytes, selected.bytes); assertEquals(offset, selected.offset)
            assertNull(left.next()); assertTrue(right.receive(selected.header)); left.finish(true)
        }
        assertNull(frame(right)); right.release(2, 3); right.release(2, 5)
        val header = checkNotNull(frame(right)).first
        assertEquals(8, header.credit); assertEquals(1L, header.sequence)
        assertFalse(left.receive(header)); assertNull(frame(right)); rejected { right.release(2, 1) }
    }
    @Test fun t846TerminalDrainsInflightWithoutStallingOthers() {
        val left = mux(); val right = mux(); grant(left, right)
        assertTrue(left.offer(2, ByteArray(8) { 1 })); assertTrue(left.offer(1, ByteArray(8) { 2 }))
        val first = checkNotNull(frame(left)).first; assertEquals(1, first.channel)
        assertTrue(right.receive(first)); right.release(first.channel, first.length)
        val pending = checkNotNull(left.next()).header; assertEquals(2, pending.channel)
        right.close(2, ConnectionWire.RESET); assertFalse(right.receive(pending))
        assertFalse(right.offer(2, byteArrayOf(4))); rejected { right.release(2, 4) }
        val credit = checkNotNull(frame(right)).first; assertEquals(ConnectionWire.CREDIT, credit.kind); left.receive(credit)
        val reset = checkNotNull(frame(right)).first; assertEquals(ConnectionWire.RESET, reset.kind); left.receive(reset)
        assertFalse(left.retired(2)); left.finish(true)
        val ack = checkNotNull(frame(left)).first; assertEquals(ConnectionWire.RESET, ack.kind); assertEquals(2L, ack.sequence)
        assertFalse(right.receive(ack)); assertFalse(left.retired(2)); rejected { left.replaceChannel(2, 10) }
        left.consumerRetired(2, 9); right.consumerRetired(2, 9)
        assertTrue(left.retired(2)); assertTrue(right.retired(2))
        rejected { right.receive(pending) }; rejected { right.close(2, ConnectionWire.CLOSE) }
        val (control, bytes) = checkNotNull(frame(left)); assertEquals(1, control.channel)
        assertArrayEquals(ByteArray(4) { 2 }, bytes); assertTrue(right.receive(control))
    }
    @Test fun t846FairnessSkipsStalledConsumer() {
        val left = mux(); val right = mux(); grant(left, right)
        val sent = IntArray(ConnectionWire.CHANNELS); val rounds = IntArray(ConnectionWire.CHANNELS)
        repeat(4096) { round ->
            for (channel in 1..ConnectionWire.CHANNELS) left.offer(channel, ByteArray(4) { channel.toByte() })
            val (header, bytes) = checkNotNull(frame(left)); assertEquals(ConnectionWire.DATA, header.kind)
            assertTrue(right.receive(header)); val index = header.channel - 1; sent[index] += bytes.size
            if (index != 1) {
                assertTrue(round - rounds[index] <= ConnectionWire.CHANNELS); rounds[index] = round
                right.release(header.channel, header.length); left.receive(checkNotNull(frame(right)).first)
            }
        }
        assertEquals(64, sent[1]); for (index in listOf(0, 2, 3, 4, 5)) assertTrue(sent[index] > 3000)
    }
    @Test fun t846InvalidBoundsAndFailedWritesRetireConnection() {
        for (capacity in listOf(Int.MIN_VALUE, 0, ConnectionWire.MAX_CREDIT + 1, Int.MAX_VALUE)) {
            rejected { ConnectionMux(1, 1, capacity, 1, 1) }
        }
        rejected { ConnectionMux(0, 1, 1, 1, 1) }; rejected { ConnectionMux(1, 0, 1, 1, 1) }
        rejected { ConnectionMux(1, 1, 1, 0, 1) }; rejected { ConnectionMux(1, 1, 1, 1, 0) }
        val left = mux(); rejected { left.finish(true) }; rejected { left.close(1, ConnectionWire.DATA) }
        for (channel in listOf(Int.MIN_VALUE, 0, ConnectionWire.CHANNELS + 1, Int.MAX_VALUE)) {
            rejected { left.offer(channel, byteArrayOf(1)) }; rejected { left.release(channel, 1) }
            rejected { left.close(channel, ConnectionWire.CLOSE) }; rejected { left.retired(channel) }
        }
        val header = checkNotNull(left.next()).header; assertEquals(0L, header.sequence)
        left.finish(false); rejected { left.next() }; assertFalse(left.offer(1, byteArrayOf(1)))
        rejected { left.receive(header) }; rejected { left.release(1, 1) }; rejected { left.close(1, ConnectionWire.RESET) }
        assertFalse(left.retired(1))
        val sender = mux(); val receiver = mux(); grant(sender, receiver)
        sender.offer(1, byteArrayOf(1)); assertNotNull(sender.next()); sender.finish(false); rejected { sender.next() }
    }
    @Test fun t846DrainValidatesBoundsSequenceAndTerminalAcknowledgement() {
        val left = ConnectionFlow(7, 9, 1, 64); val right = ConnectionFlow(7, 9, 1, 64)
        right.receive(left.send(ConnectionWire.CREDIT, 0, 64))
        val body = right.send(ConnectionWire.DATA, 16, 0)
        rejected { left.drain(body) }; val close = left.terminal(ConnectionWire.CLOSE)
        assertFalse(left.retired()); rejected { left.terminal(ConnectionWire.RESET) }
        for (invalid in listOf(body.copy(length = 65), body.copy(session = 8), body.copy(sequence = 1))) {
            rejected { left.drain(invalid) }
        }
        left.drain(body); rejected { left.drain(body) }; right.receive(close)
        val ack = right.terminal(ConnectionWire.CLOSE); assertTrue(right.retired())
        left.drain(ack); assertTrue(left.retired()); rejected { left.drain(ack) }
    }
    @Test fun t846ReplacementWaitsForPeerWriterAndConsumerRetirement() {
        val left = mux(); val right = mux(); grant(left, right)
        rejected { left.consumerRetired(3, 9) }; rejected { left.replaceChannel(3, 10) }
        left.close(3, ConnectionWire.CLOSE); rejected { left.consumerRetired(3, 8) }
        left.consumerRetired(3, 9); assertFalse(left.retired(3))
        right.receive(checkNotNull(frame(left)).first)
        val ack = checkNotNull(right.next()).header
        right.consumerRetired(3, 9); assertFalse(right.retired(3))
        left.receive(ack); assertTrue(left.retired(3)); right.finish(true); assertTrue(right.retired(3))
        for (generation in listOf(Long.MIN_VALUE, 0L, 9L)) rejected { left.replaceChannel(3, generation) }
        left.replaceChannel(3, 10); right.replaceChannel(3, 10)
        assertFalse(left.retired(3)); rejected { left.consumerRetired(3, 9) }; rejected { left.receive(ack) }
        val credit = checkNotNull(frame(left)).first; assertEquals(10L, credit.generation); assertEquals(0L, credit.sequence)
        right.receive(credit); left.receive(checkNotNull(frame(right)).first)
        assertTrue(left.offer(3, ByteArray(4) { 7 })); val (header, bytes) = checkNotNull(frame(left))
        assertEquals(10L, header.generation); assertArrayEquals(ByteArray(4) { 7 }, bytes)
        assertTrue(right.receive(header)); right.release(3, 4)
        for (channel in listOf(0, 7, Int.MAX_VALUE)) {
            rejected { left.consumerRetired(channel, 10) }; rejected { left.replaceChannel(channel, 11) }
        }
    }
}
