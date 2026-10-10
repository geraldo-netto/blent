package com.blent.connection

import com.blent.connection.ConnectionManagement.Resource

/** Five fixed lazy rings. Outer Session owns resource authority and serializes
 * calls. A Read is one exact native-output loan, never proof of its completion.
 * Its array range remains unchanged while pending, including concurrent pushes.
 * Caller must not mutate it and must acknowledge only after write AND flush.
 * Full rings apply backpressure; no copying, eviction or early credit return. */
internal class ConnectionChannelBuffers(private val capacity: Int) {
    private class Ring(capacity: Int) {
        val bytes = ByteArray(capacity)
        var head = 0
        var retained = 0
        var pending: ConnectionChannelRead? = null
    }
    init { require(capacity in 1..ConnectionWire.MAX_CREDIT) }
    private val rings = arrayOfNulls<Ring>(5)

    fun install(resource: Resource) {
        val channels = ConnectionChannels.members(resource)
        check(channels.all { rings[it - 1] == null }) { "Media buffers still owned" }
        for (channel in channels) rings[channel - 1] = Ring(capacity)
    }
    private fun ring(channel: Int): Ring {
        require(channel in ConnectionWire.CONTROL..ConnectionWire.SPEAKER)
        return checkNotNull(rings[channel - 1]) { "Media consumer not installed" }
    }
    fun push(channel: Int, bytes: ByteArray, offset: Int = 0, length: Int = bytes.size - offset) {
        require(offset >= 0 && offset <= bytes.size && length >= 0 && length <= bytes.size - offset)
        val ring = ring(channel)
        require(bytes !== ring.bytes) { "Receive input aliases owned ring storage" }
        require(length <= capacity - ring.retained) { "Consumer receive window exceeded" }
        val tail = (ring.head + ring.retained) % capacity
        val first = minOf(length, capacity - tail)
        bytes.copyInto(ring.bytes, tail, offset, offset + first)
        bytes.copyInto(ring.bytes, 0, offset + first, offset + length)
        ring.retained += length
    }
    fun read(channel: Int, maxBytes: Int): ConnectionChannelRead? {
        require(maxBytes in 1..ConnectionWire.MAX_PAYLOAD)
        val ring = ring(channel)
        if (ring.pending != null || ring.retained == 0) return null
        val length = minOf(maxBytes, ring.retained, capacity - ring.head)
        return ConnectionChannelRead(ring.bytes, ring.head, length).also { ring.pending = it }
    }
    fun consumed(channel: Int, receipt: ConnectionChannelRead) {
        val ring = ring(channel)
        require(ring.pending === receipt) { "Foreign or retired consumer receipt" }
        ring.head = (ring.head + receipt.length) % capacity
        ring.retained -= receipt.length
        ring.pending = null
    }
    fun readable(channel: Int): Boolean {
        val ring = ring(channel)
        return ring.retained > 0 && ring.pending == null
    }
    /** Caller has observed actual consumer/native retirement for every resource
     * channel. Cancellation or timeout alone must NEVER call this method.
     * Old receipts retain their arrays; reinstall allocates distinct rings. */
    fun retire(resource: Resource) {
        for (channel in ConnectionChannels.members(resource)) rings[channel - 1] = null
    }
}
