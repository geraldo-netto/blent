package com.blent.connection

/** T881/T1759 keeps exact frame ownership through partial prefix progress and
 * final flush. A full transaction completion releases the original frame; errors
 * retire the connection, never retry it as a new frame. Native deadlines and
 * actual worker/process retirement remain the supervisor's responsibility. */
internal class ConnectionFrameWriter(timeoutMs: Long) : ConnectionDeadlineSource {
    private val clock = ConnectionFrameClock(timeoutMs)
    private val headerBytes = ByteArray(ConnectionWire.HEADER_BYTES)
    private var active: ConnectionFrame? = null
    private var position = 0
    private var native: ConnectionWriteTransaction? = null

    fun begin(frame: ConnectionFrame, now: Long) {
        clock.check(now)
        check(active == null && native == null)
        ConnectionWire.validate(frame.header)
        require(frame.offset in 0..frame.bytes.size)
        require(frame.length == frame.header.length && frame.length <= frame.bytes.size - frame.offset)
        clock.start(now)
        ConnectionWire.encode(frame.header, headerBytes)
        active = frame
        position = 0
    }
    fun write(now: Long): ConnectionWriteTransaction? {
        clock.check(now)
        val frame = active ?: return null
        if (native != null) return null
        val body = body(frame)
        val transaction = if (position < headerBytes.size) {
            ConnectionWriteTransaction(ConnectionBuffer(headerBytes, position, headerBytes.size - position), body)
        } else ConnectionWriteTransaction(checkNotNull(body))
        native = transaction
        return transaction
    }
    private fun body(frame: ConnectionFrame): ConnectionBuffer? {
        val offset = maxOf(0, position - headerBytes.size)
        if (offset == frame.length) return null
        return ConnectionBuffer(frame.bytes, frame.offset + offset, frame.length - offset)
    }
    fun writeComplete(loan: ConnectionWriteTransaction, count: Int, now: Long): ConnectionFrame? {
        require(native === loan) { "Foreign or retired write transaction" }
        native = null
        try {
            clock.check(now)
            require(count in 0..loan.length) { "Invalid write extent" }
            position += count
            if (count != loan.length) return null
            val frame = checkNotNull(active)
            active = null
            clock.clear()
            return frame
        } catch (error: Exception) { clock.cancel(); throw error }
    }
    fun nativePending(): Boolean = native != null
    fun tick(now: Long) = clock.check(now)
    fun cancel() = clock.cancel()
    override fun nextDeadline(): ConnectionDeadline = clock.nextDeadline()
}
