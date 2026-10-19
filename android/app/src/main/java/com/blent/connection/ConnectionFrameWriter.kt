package com.blent.connection

/** T881 one writer preserves offsets and payload ownership through partial
 * writes and flush. Its caller calls mux.finish only after flushComplete returns
 * the exact frame. Failed/cancelled native work retires the whole connection;
 * it is never retried as a new frame. Native deadlines belong to the supervisor. */
internal class ConnectionFrameWriter(timeoutMs: Long) : ConnectionDeadlineSource {
    private val clock = ConnectionFrameClock(timeoutMs)
    private val headerBytes = ByteArray(ConnectionWire.HEADER_BYTES)
    private var active: ConnectionFrame? = null
    private var headerOffset = 0
    private var bodyOffset = 0
    private var native: ConnectionBuffer? = null
    private var flushing: ConnectionFlush? = null

    fun begin(frame: ConnectionFrame, now: Long) {
        clock.check(now)
        check(active == null && native == null && flushing == null)
        ConnectionWire.validate(frame.header)
        require(frame.offset in 0..frame.bytes.size)
        require(frame.length == frame.header.length && frame.length <= frame.bytes.size - frame.offset)
        clock.start(now)
        ConnectionWire.encode(frame.header, headerBytes)
        active = frame
        headerOffset = 0
        bodyOffset = 0
    }
    fun write(now: Long): ConnectionBuffer? {
        clock.check(now)
        val frame = active ?: return null
        if (native != null || flushing != null) return null
        val slice = when {
            headerOffset < headerBytes.size -> ConnectionBuffer(headerBytes, headerOffset, headerBytes.size - headerOffset)
            bodyOffset < frame.length -> ConnectionBuffer(frame.bytes, frame.offset + bodyOffset, frame.length - bodyOffset)
            else -> return null
        }
        native = slice
        return slice
    }
    fun writeComplete(loan: ConnectionBuffer, count: Int, now: Long) {
        require(native === loan) { "Foreign or retired write" }
        native = null
        try {
            clock.check(now)
            require(count in 0..loan.length) { "Invalid write extent" }
            if (headerOffset < headerBytes.size) headerOffset += count else bodyOffset += count
        } catch (error: Exception) { clock.cancel(); throw error }
    }
    fun flush(now: Long): ConnectionFlush? {
        clock.check(now)
        val frame = active ?: return null
        if (native != null || flushing != null) return null
        if (headerOffset != headerBytes.size || bodyOffset != frame.length) return null
        return ConnectionFlush().also { flushing = it }
    }
    fun flushComplete(loan: ConnectionFlush, now: Long): ConnectionFrame {
        require(flushing === loan) { "Foreign or retired flush" }
        flushing = null
        clock.check(now)
        val frame = checkNotNull(active)
        active = null
        clock.clear()
        return frame
    }
    fun nativePending(): Boolean = native != null || flushing != null
    fun tick(now: Long) = clock.check(now)
    fun cancel() = clock.cancel()
    override fun nextDeadline(): ConnectionDeadline = clock.nextDeadline()
}
