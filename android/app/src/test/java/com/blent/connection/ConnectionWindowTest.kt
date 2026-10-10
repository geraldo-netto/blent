package com.blent.connection

import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [34])
class ConnectionWindowTest {
    private fun rejected(action: () -> Unit) { assertThrows(Exception::class.java) { action() } }
    private fun bytes(hex: String) = hex.chunked(2).map { it.toInt(16).toByte() }.toByteArray()
    private fun vectors(): JSONObject = javaClass.classLoader!!.getResourceAsStream("connection-window-v1.json")!!
        .bufferedReader().use { JSONObject(it.readText()) }

    @Test fun t881IndependentGoldenWindowRecords() {
        val valid = vectors().getJSONArray("valid")
        for (index in 0 until valid.length()) {
            val record = valid.getJSONObject(index)
            val wire = bytes(record.getString("hex"))
            val capacity = record.getInt("capacity")
            val host = record.getBoolean("host")
            assertEquals(capacity, ConnectionWindow.decode(wire, host))
            assertArrayEquals(wire, ConnectionWindow.encode(host, capacity))
            rejected { ConnectionWindow.decode(wire, !host) }
        }
        val invalid = vectors().getJSONArray("invalid")
        for (index in 0 until invalid.length()) {
            val record = invalid.getJSONObject(index)
            rejected { ConnectionWindow.decode(bytes(record.getString("hex")), record.getBoolean("host")) }
        }
    }
    @Test fun t881WindowCapacityBoundsAndEveryPrefixMutation() {
        val values = listOf(Int.MIN_VALUE, -1, 0, 4095, 4096, 4097, 65536, 1048576, 1048577, Int.MAX_VALUE)
        for (host in listOf(false, true)) for (value in values) checkCapacity(host, value)
        val valid = ConnectionWindow.encode(true, 4096)
        for (position in 0..7) for (value in 0..255) {
            if (value == valid[position].toInt()) continue
            val changed = valid.copyOf().apply { this[position] = value.toByte() }
            rejected { ConnectionWindow.decode(changed, true) }
        }
        for (size in listOf(0, 1, 11, 13, 4096, 65536)) rejected { ConnectionWindow.decode(valid.copyOf(size), true) }
    }
    private fun checkCapacity(host: Boolean, value: Int) {
        if (value in 4096..1048576) {
            ConnectionWindow.validate(value)
            assertEquals(value, ConnectionWindow.decode(ConnectionWindow.encode(host, value), host))
        } else {
            rejected { ConnectionWindow.validate(value) }
            rejected { ConnectionWindow.encode(host, value) }
        }
    }
    @Test fun t881SelectionNeverExpandsEitherCapacity() {
        val values = listOf(0, 4095, 4096, 4097, 65536, 1048576, Int.MAX_VALUE)
        for (proposed in values) for (local in values) selection(proposed, local)
    }
    private fun selection(proposed: Int, local: Int) {
        if (proposed in 4096..1048576 && local in 4096..1048576) {
            val selected = ConnectionWindow.select(proposed, local)
            assertEquals(minOf(proposed, local), selected)
            assertEquals(selected, ConnectionWindow.accepted(proposed, selected))
        } else rejected { ConnectionWindow.select(proposed, local) }
        if (proposed !in 4096..1048576 || local !in 4096..proposed) rejected { ConnectionWindow.accepted(proposed, local) }
    }
    @Test fun t881EveryReadWriteSplitRequiresExactFlushReceipt() {
        val proposal = ConnectionWindow.encode(true, 65536)
        for (readSplit in 0..12) for (writeSplit in 1..12) split(proposal, readSplit, writeSplit)
    }
    private fun split(proposal: ByteArray, readSplit: Int, writeSplit: Int) {
        val state = ConnectionWindowNegotiation(8192, 10, 100)
        assertNull(state.selected())
        assertNull(state.output(10))
        assertEquals(readSplit, state.feed(proposal, 0, readSplit, 11))
        if (readSplit < 12) assertEquals(12 - readSplit, state.feed(proposal, readSplit, 12 - readSplit, 12))
        val first = state.output(13)!!
        assertNull(state.output(13))
        assertEquals(0, first.offset); assertEquals(12, first.length)
        assertArrayEquals(ConnectionWindow.encode(false, 8192), first.bytes)
        assertNull(state.flush(13))
        state.wrote(first, writeSplit, 14)
        finishOutput(state, writeSplit)
        assertNull(state.selected())
        val flush = state.flush(16)!!
        assertNull(state.flush(16))
        assertEquals(8192, state.flushed(flush, 17))
        assertEquals(8192, state.selected())
    }
    private fun finishOutput(state: ConnectionWindowNegotiation, written: Int) {
        if (written == 12) { assertNull(state.output(15)); return }
        val remaining = state.output(15)!!
        assertEquals(written, remaining.offset); assertEquals(12 - written, remaining.length)
        state.wrote(remaining, remaining.length, 15)
    }
    @Test fun t881CoalescedInputLeavesFollowingFrameWithCaller() {
        val state = ConnectionWindowNegotiation(65536, 0, 100)
        val bytes = ConnectionWindow.encode(true, 4096) + ByteArray(1048576) { 7 }
        assertEquals(12, state.feed(bytes, 0, bytes.size, 1))
        assertEquals(4096, ConnectionWindow.decode(state.output(1)!!.bytes, false))
        rejected { state.feed(bytes, 12, 1, 2) }
        assertNull(state.selected())
    }
    @Test fun t881MalformedProposalAndInvalidSlicesPoisonNegotiation() {
        val cases = listOf(-1 to 1, 0 to -1, 13 to 0, 11 to 2, Int.MAX_VALUE to Int.MAX_VALUE)
        for ((offset, length) in cases) {
            val state = ConnectionWindowNegotiation(4096, 0, 100)
            rejected { state.feed(ByteArray(12), offset, length, 1) }
            rejected { state.feed(ConnectionWindow.encode(true, 4096), 0, 12, 2) }
        }
        val state = ConnectionWindowNegotiation(4096, 0, 100)
        rejected { state.feed(ConnectionWindow.encode(false, 4096), 0, 12, 1) }
        rejected { state.output(2) }
        rejected { state.flush(2) }
    }
    @Test fun t881AbsoluteDeadlineAndClockRegressionCoverAllPhases() {
        for (phase in 0..2) expired(phase)
        val state = ConnectionWindowNegotiation(4096, 10, 100)
        state.checkDeadline(20)
        rejected { state.checkDeadline(19) }
        rejected { state.checkDeadline(21) }
        for ((now, timeout) in listOf(-1L to 1L, 0L to 0L, 0L to 60001L, Long.MAX_VALUE to 1L)) {
            rejected { ConnectionWindowNegotiation(4096, now, timeout) }
        }
        rejected { ConnectionWindowNegotiation(4095, 0, 1) }
    }
    private fun expired(phase: Int) {
        val state = ConnectionWindowNegotiation(4096, 0, 10)
        if (phase > 0) state.feed(ConnectionWindow.encode(true, 4096), 0, 12, 8)
        if (phase > 1) state.wrote(state.output(8)!!, 12, 9)
        state.checkDeadline(9)
        rejected { state.checkDeadline(10) }
        rejected { state.output(11) }
        assertNull(state.selected())
    }
    @Test fun t881StaleWriteAndFlushReceiptsCannotAdvanceNegotiation() {
        val first = ready(); val second = ready()
        val loan = first.output(1)!!
        rejected { second.wrote(loan, 1, 2) }
        first.wrote(loan, 1, 2)
        val remaining = first.output(3)!!
        rejected { first.wrote(loan, 1, 4) }
        assertTrue(first.nativePending())
        assertEquals(1, remaining.offset)
        first.wrote(remaining, 11, 4)
        assertFalse(first.nativePending())
        for (count in listOf(-1, 0, 13, Int.MAX_VALUE)) {
            val state = ready(); rejected { state.wrote(state.output(1)!!, count, 2) }
        }
        val a = flushing(); val b = flushing()
        val receipt = a.flush(2)!!
        rejected { b.flushed(receipt, 3) }
        a.flushed(receipt, 3)
        rejected { a.flushed(receipt, 4) }
    }
    private fun ready() = ConnectionWindowNegotiation(4096, 0, 100).apply {
        feed(ConnectionWindow.encode(true, 8192), 0, 12, 1)
    }
    private fun flushing() = ready().apply { wrote(output(1)!!, 12, 2) }
    @Test fun t881CancelRetainsLoanButNeverPublishesCapacity() {
        val state = ready(); val loan = state.output(1)!!
        state.cancel(); state.cancel()
        assertTrue(state.nativePending())
        assertArrayEquals(ConnectionWindow.encode(false, 4096), loan.bytes)
        rejected { state.wrote(loan, 12, 2) }
        assertFalse(state.nativePending())
        assertNull(state.selected())
        val finished = flushing(); val flush = finished.flush(2)!!
        finished.cancel(); assertTrue(finished.nativePending())
        rejected { finished.flushed(flush, 3) }
        assertFalse(finished.nativePending())
    }
    @Test fun t881BoundedRandomInputHasNoOutOfBoundsConsumption() {
        val random = java.util.Random(881)
        repeat(4096) {
            val bytes = ByteArray(random.nextInt(32)).apply { random.nextBytes(this) }
            val state = ConnectionWindowNegotiation(4096, 0, 100)
            try {
                val consumed = state.feed(bytes, 0, bytes.size, 1)
                assertEquals(minOf(12, bytes.size), consumed)
                assertNull(state.selected())
            } catch (_: IllegalArgumentException) { rejected { state.checkDeadline(2) } }
            catch (_: IllegalStateException) { rejected { state.checkDeadline(2) } }
            state.cancel()
        }
    }
}
