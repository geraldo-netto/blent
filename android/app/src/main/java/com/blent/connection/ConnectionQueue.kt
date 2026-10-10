package com.blent.connection

import java.util.ArrayDeque

/** Owns admitted arrays until their last slice is written. Callers transfer array
 * ownership on successful offer, must not mutate retained slices, and serialize
 * all operations through the connection owner. No native work occurs here. */
internal class ConnectionQueue(private val capacity: Int, private val messages: Int, private val quantum: Int) {
    private class Pending(val bytes: ByteArray, var offset: Int = 0)
    data class Slice(val channel: Int, val bytes: ByteArray, val offset: Int, val length: Int)
    private val queues = Array(ConnectionWire.CHANNELS) { ArrayDeque<Pending>() }
    private val retained = IntArray(ConnectionWire.CHANNELS)
    private var cursor = 0
    private var active: Slice? = null

    init {
        require(capacity in 1..ConnectionWire.MAX_CREDIT)
        require(messages in 1..1024)
        require(quantum in 1..ConnectionWire.MAX_PAYLOAD)
    }

    /** False returns ownership unchanged; admission never copies payload bytes. */
    fun offer(channel: Int, bytes: ByteArray): Boolean {
        require(channel in 1..ConnectionWire.CHANNELS)
        val index = channel - 1
        if (bytes.isEmpty() || bytes.size > ConnectionWire.MAX_PAYLOAD ||
            bytes.size > capacity - retained[index] || queues[index].size >= messages) return false
        queues[index].addLast(Pending(bytes))
        retained[index] += bytes.size
        return true
    }

    fun next(credit: IntArray): Slice? {
        require(credit.size == ConnectionWire.CHANNELS && credit.all { it in 0..ConnectionWire.MAX_CREDIT })
        if (active != null) return null
        for (distance in queues.indices) {
            val index = (cursor + distance) % queues.size
            if (credit[index] == 0) continue
            val item = queues[index].peekFirst() ?: continue
            return reserve(index, item, credit[index])
        }
        return null
    }

    private fun reserve(index: Int, item: Pending, credit: Int): Slice {
        val length = minOf(item.bytes.size - item.offset, quantum, credit)
        return Slice(index + 1, item.bytes, item.offset, length).also {
            active = it
            cursor = (index + 1) % queues.size
        }
    }

    /** Unsent cancellation preserves data; partial native writes retire stream. */
    fun finish(sent: Boolean) {
        val selected = checkNotNull(active) { "No reserved write" }
        if (sent) advance(selected)
        active = null
    }

    private fun advance(selected: Slice) {
        val index = selected.channel - 1
        val item = queues[index].first
        item.offset += selected.length
        if (item.offset == item.bytes.size) {
            retained[index] -= item.bytes.size
            queues[index].removeFirst()
        }
    }

    fun clear(channel: Int) {
        require(channel in 1..ConnectionWire.CHANNELS)
        check(active?.channel != channel) { "Writer still owns channel" }
        queues[channel - 1].clear()
        retained[channel - 1] = 0
    }
}
