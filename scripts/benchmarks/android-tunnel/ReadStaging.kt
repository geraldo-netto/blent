package com.blent.connection

/** T1757 benchmark-only bounded read-ahead. One exact native loan or retained
 * prefix, never both. Copying into production framing preserves its independent
 * validation and completed-frame lifetime; no production default changes. */
internal class ReadStaging(capacity: Int) {
    init { require(capacity in 1..ConnectionWire.MAX_PAYLOAD) }
    private val bytes = ByteArray(capacity)
    private var cursor = 0
    private var end = 0
    private var native: ConnectionBuffer? = null
    var copied = 0L; private set
    fun read(): ConnectionBuffer {
        check(native == null && cursor == end)
        return ConnectionBuffer(bytes, 0, bytes.size).also { native = it }
    }
    fun complete(loan: ConnectionBuffer, count: Int) {
        require(native === loan)
        require(count in 1..loan.length)
        native = null
        cursor = 0
        end = count
    }
    fun copyTo(loan: ConnectionBuffer): Int {
        check(native == null)
        require(loan.offset in 0..loan.bytes.size && loan.length > 0)
        require(loan.length <= loan.bytes.size - loan.offset)
        val count = minOf(loan.length, end - cursor)
        bytes.copyInto(loan.bytes, loan.offset, cursor, cursor + count)
        cursor += count
        copied += count
        return count
    }
    fun empty(): Boolean = cursor == end
}

internal class MeasuredFrameReader(aheadBytes: Int) {
    init { require(aheadBytes in 0..ConnectionWire.MAX_PAYLOAD) }
    private val framing = ConnectionFrameReader(5000)
    private val staging = if (aheadBytes == 0) null else ReadStaging(aheadBytes)
    val copied: Long get() = staging?.copied ?: 0
    fun next(io: MeasuredIo): ConnectionFrame {
        while (true) {
            framing.frame()?.let { return it }
            val loan = checkNotNull(framing.read(io.now()))
            val count = staging?.let { stagedRead(io, it, loan) }
                ?: io.perform(ConnectionIoRequest.Read(loan))
            framing.readComplete(loan, count, io.now())
        }
    }
    private fun stagedRead(io: MeasuredIo, staged: ReadStaging, destination: ConnectionBuffer): Int {
        if (staged.empty()) {
            val loan = staged.read()
            staged.complete(loan, io.perform(ConnectionIoRequest.Read(loan)))
        }
        return staged.copyTo(destination)
    }
    fun release(frame: ConnectionFrame) = framing.release(frame)
}
