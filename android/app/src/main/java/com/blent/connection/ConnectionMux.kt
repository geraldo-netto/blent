package com.blent.connection

/** Single connection owner serializes calls. Native I/O and authentication live
 * in adapters. One reserved writer; metadata never depends on DATA credit. */
internal class ConnectionMux(private val session: Long, generation: Long, private val capacity: Int, messages: Int, quantum: Int) {
    data class Frame(val header: ConnectionWire.Header, val bytes: ByteArray, val offset: Int, val length: Int)
    private val generations = LongArray(ConnectionWire.CHANNELS) { generation }
    private val consumerRetired = BooleanArray(ConnectionWire.CHANNELS)
    private val flows = Array(ConnectionWire.CHANNELS) { ConnectionFlow(session, generation, it + 1, capacity) }
    private val queue = ConnectionQueue(capacity, messages, quantum)
    private val enabled = BooleanArray(ConnectionWire.CHANNELS) { it == ConnectionWire.CHANNELS - 1 }
    private val credit = IntArray(ConnectionWire.CHANNELS) { if (enabled[it]) capacity else 0 }
    private val terminal = arrayOfNulls<ConnectionWire.Header>(ConnectionWire.CHANNELS)
    private val closing = BooleanArray(ConnectionWire.CHANNELS)
    private var active: ConnectionWire.Header? = null
    private var metadataCursor = 0
    private var metadataFirst = true
    private var failed = false
    private val empty = ByteArray(0)

    /** Caller serializes authenticated management commit completion with this
     * operation: sender finishes the whole COMMIT record before activation;
     * receiver installs the lease and activates while handling COMMIT, before
     * accepting another envelope. A generation alone is never authority. */
    fun activate(channel: Int, generation: Long) {
        require(channel in 1..ConnectionWire.CHANNELS)
        val index = channel - 1
        check(!failed && !closing[index] && !enabled[index] && generations[index] == generation)
        enabled[index] = true
        credit[index] = capacity
    }

    fun offer(channel: Int, bytes: ByteArray): Boolean {
        require(channel in 1..ConnectionWire.CHANNELS)
        if (failed || closing[channel - 1] || !enabled[channel - 1]) return false
        return queue.offer(channel, bytes)
    }

    /** Closing one member of a resource must not close its terminal twice. */
    fun acceptsData(channel: Int, generation: Long): Boolean {
        require(channel in 1..ConnectionWire.CHANNELS)
        val index = channel - 1
        return !failed && enabled[index] && !closing[index] && generations[index] == generation
    }

    /** False DATA must be drained, never delivered. Complete the bounded body
     * before accepting another header or replacing a retired channel owner. */
    fun receive(header: ConnectionWire.Header): Boolean {
        check(!failed)
        ConnectionWire.validate(header)
        val index = header.channel - 1
        if (closing[index]) { flows[index].drain(header); return false }
        check(enabled[index] || header.kind == ConnectionWire.CLOSE || header.kind == ConnectionWire.RESET)
        flows[index].receive(header)
        if (header.kind == ConnectionWire.CLOSE || header.kind == ConnectionWire.RESET) close(header.channel, header.kind)
        return header.kind == ConnectionWire.DATA
    }

    fun release(channel: Int, bytes: Int) {
        require(channel in 1..ConnectionWire.CHANNELS)
        val index = channel - 1
        check(!failed && !closing[index] && enabled[index])
        flows[index].release(bytes)
        credit[index] += bytes
    }

    /** Terminal takes the final sequence after any active payload. No further
     * DATA/CREDIT can overtake it; authority is revoked before native cleanup. */
    fun close(channel: Int, kind: Int) {
        require(channel in 1..ConnectionWire.CHANNELS)
        check(!failed)
        val index = channel - 1
        check(!closing[index])
        val header = flows[index].terminal(kind)
        terminal[index] = header
        closing[index] = true
        enabled[index] = false
        credit[index] = 0
        if (active?.channel != channel) queue.clear(channel)
    }

    private fun metadata(): ConnectionWire.Header? {
        for (distance in flows.indices) {
            val index = (metadataCursor + distance) % flows.size
            val header = takeMetadata(index) ?: continue
            metadataCursor = (index + 1) % flows.size
            metadataFirst = false
            active = header
            return header
        }
        return null
    }

    private fun takeMetadata(index: Int): ConnectionWire.Header? {
        terminal[index]?.let { terminal[index] = null; return it }
        if (credit[index] == 0) return null
        return flows[index].send(ConnectionWire.CREDIT, 0, credit[index]).also { credit[index] = 0 }
    }

    fun next(): Frame? {
        check(!failed)
        if (active != null) return null
        if (metadataFirst) metadata()?.let { return Frame(it, empty, 0, 0) }
        val slice = queue.next(IntArray(flows.size) { flows[it].available() })
        if (slice != null) {
            val header = flows[slice.channel - 1].send(ConnectionWire.DATA, slice.length, 0)
            active = header
            metadataFirst = true
            return Frame(header, slice.bytes, slice.offset, slice.length)
        }
        return metadata()?.let { Frame(it, empty, 0, 0) }
    }

    /** Must follow actual writer retirement. Failure poisons the connection;
     * never retry a frame whose bytes might already have reached the peer. */
    fun finish(success: Boolean) {
        val header = checkNotNull(active)
        active = null
        if (header.kind == ConnectionWire.DATA) queue.finish(success)
        if (closing[header.channel - 1]) queue.clear(header.channel)
        failed = failed || !success
    }

    /** Confirm both local bridge I/O directions actually retired their buffers. */
    fun consumerRetired(channel: Int, generation: Long) {
        require(channel in 1..ConnectionWire.CHANNELS)
        val index = channel - 1
        check(!failed && closing[index] && generations[index] == generation)
        consumerRetired[index] = true
    }

    /** Generation is agreed through authenticated management, after old body drain. */
    fun replaceChannel(channel: Int, generation: Long) {
        require(channel in 1..ConnectionWire.CHANNELS)
        val index = channel - 1
        check(retired(channel) && generation > generations[index])
        val flow = ConnectionFlow(session, generation, channel, capacity)
        flows[index] = flow; generations[index] = generation
        consumerRetired[index] = false; closing[index] = false; enabled[index] = false; credit[index] = 0
    }

    fun retired(channel: Int): Boolean {
        require(channel in 1..ConnectionWire.CHANNELS)
        val index = channel - 1
        return !failed && consumerRetired[index] && flows[index].retired() && terminal[index] == null && active?.channel != channel
    }
}
