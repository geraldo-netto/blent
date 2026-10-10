package com.blent.connection

import java.nio.ByteBuffer
import java.nio.ByteOrder

/** Authenticated BLCP v1 record. Parsing chooses capacity, never identity or a
 * media lease. Host proposes; tablet selects no more than either local bound. */
internal object ConnectionWindow {
    const val BYTES = 12
    const val MIN_CAPACITY = 4096

    fun validate(capacity: Int) { require(capacity in MIN_CAPACITY..ConnectionWire.MAX_CREDIT) }

    fun encode(host: Boolean, capacity: Int): ByteArray {
        validate(capacity)
        return ByteBuffer.allocate(BYTES).order(ByteOrder.BIG_ENDIAN).apply {
            putInt(0x424c4350); put(1); put(if (host) 1.toByte() else 0.toByte()); putShort(0); putInt(capacity)
        }.array()
    }

    fun decode(bytes: ByteArray, expectHost: Boolean): Int {
        require(bytes.size == BYTES)
        val input = ByteBuffer.wrap(bytes).order(ByteOrder.BIG_ENDIAN)
        require(input.int == 0x424c4350 && input.get() == 1.toByte())
        require(input.get() == (if (expectHost) 1.toByte() else 0.toByte()) && input.short == 0.toShort())
        return input.int.also(::validate)
    }

    fun select(proposed: Int, local: Int): Int {
        validate(proposed); validate(local)
        return minOf(proposed, local)
    }

    fun accepted(proposed: Int, selected: Int): Int {
        validate(proposed); validate(selected)
        require(selected <= proposed)
        return selected
    }
}

/** One serialized tablet setup owner, after mutual TLS and before framing.
 * Partial reads consume only this record; following bytes remain with caller.
 * Native adapters must flush the entire reply and return its exact receipt
 * before selected() publishes a capacity. One absolute caller-clock deadline
 * spans reading, writing and flush; it is never extended by byte progress.
 * output/flush return a loan only once, until its exact completion callback.
 * Cancellation poisons admission but retains borrowed arrays. It does not prove
 * native I/O retirement; the caller must retain its worker until actual exit. */
internal class ConnectionWindowNegotiation(private val local: Int, now: Long, timeoutMillis: Long) {
    class Write internal constructor(val bytes: ByteArray, val offset: Int, val length: Int)
    class Flush internal constructor()
    private enum class Stage { READING, WRITING, FLUSHING, COMPLETE }
    init {
        ConnectionWindow.validate(local)
        require(now >= 0 && timeoutMillis in 1..60000 && now <= Long.MAX_VALUE - timeoutMillis)
    }
    private val deadline = now + timeoutMillis
    private var clock = now
    private val proposal = ByteArray(ConnectionWindow.BYTES)
    private var received = 0
    private var reply: ByteArray? = null
    private var written = 0
    private var capacity = 0
    private var stage = Stage.READING
    private var failed = false
    private var write: Write? = null
    private var flush: Flush? = null

    private fun requireCurrent(condition: Boolean) {
        if (!condition) { failed = true; error("Window negotiation is invalid, expired or retired") }
    }

    fun checkDeadline(now: Long) {
        requireCurrent(!failed && stage != Stage.COMPLETE && now >= clock && now < deadline)
        clock = now
    }

    fun feed(bytes: ByteArray, offset: Int, length: Int, now: Long): Int {
        checkDeadline(now)
        requireCurrent(stage == Stage.READING)
        requireCurrent(offset >= 0 && offset <= bytes.size && length >= 0 && length <= bytes.size - offset)
        val count = minOf(length, proposal.size - received)
        bytes.copyInto(proposal, received, offset, offset + count)
        received += count
        if (received == proposal.size) prepare()
        return count
    }

    private fun prepare() {
        try {
            capacity = ConnectionWindow.select(ConnectionWindow.decode(proposal, true), local)
            reply = ConnectionWindow.encode(false, capacity)
            stage = Stage.WRITING
        } catch (failure: Exception) { failed = true; throw failure }
    }

    fun output(now: Long): Write? {
        checkDeadline(now)
        if (stage != Stage.WRITING || write != null) return null
        return Write(checkNotNull(reply), written, ConnectionWindow.BYTES - written).also { write = it }
    }

    fun wrote(loan: Write, count: Int, now: Long) {
        require(write === loan) { "Foreign or retired window write" }
        write = null
        checkDeadline(now)
        requireCurrent(stage == Stage.WRITING && count in 1..loan.length)
        written += count
        if (written == ConnectionWindow.BYTES) stage = Stage.FLUSHING
    }

    fun flush(now: Long): Flush? {
        checkDeadline(now)
        if (stage != Stage.FLUSHING || flush != null) return null
        return Flush().also { flush = it }
    }

    fun flushed(receipt: Flush, now: Long): Int {
        require(flush === receipt) { "Foreign or retired window flush" }
        flush = null
        checkDeadline(now)
        requireCurrent(stage == Stage.FLUSHING)
        stage = Stage.COMPLETE
        return capacity
    }

    fun nativePending(): Boolean = write != null || flush != null
    fun selected(): Int? = if (!failed && stage == Stage.COMPLETE) capacity else null
    fun cancel() { failed = true }
}
