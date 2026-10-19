package com.blent.connection

/** T881 one bounded framed reader. Native reads target retained arrays directly;
 * at most one loan and one completed frame exist. A completed frame must be
 * dispatched/released before another read. Resource credit belongs to Session,
 * not this transport staging buffer. No stream, task or OS behavior lives here. */
internal class ConnectionFrameReader(timeoutMs: Long) : ConnectionDeadlineSource {
    private val clock = ConnectionFrameClock(timeoutMs)
    private val headerBytes = ByteArray(ConnectionWire.HEADER_BYTES)
    private val bodyBytes = ByteArray(ConnectionWire.MAX_PAYLOAD)
    private var headerOffset = 0
    private var bodyOffset = 0
    private var header: ConnectionWire.Header? = null
    private var native: ConnectionBuffer? = null
    private var completed: ConnectionFrame? = null
    private var claimed = false

    fun read(now: Long): ConnectionBuffer? {
        clock.check(now)
        if (native != null || completed != null) return null
        val parsed = header
        return if (parsed == null) {
            ConnectionBuffer(headerBytes, headerOffset, headerBytes.size - headerOffset)
        } else {
            ConnectionBuffer(bodyBytes, bodyOffset, parsed.length - bodyOffset)
        }.also { native = it }
    }

    /** Call only after the named native read returned, including EOF/errors.
     * A late callback retires its exact loan but cannot reopen a cancelled reader.
     * Zero progress preserves offsets and the original active-frame deadline. */
    fun readComplete(loan: ConnectionBuffer, count: Int, now: Long) {
        require(native === loan) { "Foreign or retired read" }
        native = null
        try {
            clock.check(now)
            require(count in 0..loan.length) { "EOF or invalid read extent" }
            if (count == 0) return
            if (header == null) receiveHeader(count, now) else bodyOffset += count
            val parsed = header ?: return
            if (bodyOffset == parsed.length) {
                completed = ConnectionFrame(parsed, bodyBytes, 0, parsed.length)
                clock.clear()
            }
        } catch (error: Exception) { clock.cancel(); throw error }
    }
    private fun receiveHeader(count: Int, now: Long) {
        if (headerOffset == 0) clock.start(now)
        headerOffset += count
        if (headerOffset == headerBytes.size) header = ConnectionWire.decode(headerBytes)
    }
    fun frame(): ConnectionFrame? {
        if (clock.failed || claimed) return null
        return completed?.also { claimed = true }
    }
    fun release(frame: ConnectionFrame) {
        require(claimed && completed === frame) { "Foreign or retired frame" }
        completed = null
        claimed = false
        header = null
        headerOffset = 0
        bodyOffset = 0
    }
    fun nativePending(): Boolean = native != null
    fun tick(now: Long) = clock.check(now)
    fun cancel() = clock.cancel()
    override fun nextDeadline(): ConnectionDeadline = clock.nextDeadline()
}
