package com.blent.connection

/** One bounded record per feed result, even for a large coalesced input. The
 * caller advances by consumed before feeding its remainder. Malformed/truncated
 * wire poisons this owner; neither message boundaries nor resync are guessed. */
internal class ManagementDecoder {
    data class Feed(val consumed: Int, val message: ConnectionManagement.Message?)
    private val buffer = ByteArray(ConnectionManagement.MAX_RECORD_BYTES)
    private var used = 0
    private var target = ConnectionManagement.HEADER_BYTES
    private var ended = false
    private var failed = false

    fun feed(bytes: ByteArray, offset: Int = 0, length: Int = bytes.size - offset): Feed {
        require(offset >= 0 && length >= 0 && offset <= bytes.size && length <= bytes.size - offset)
        check(!ended && !failed)
        try { return accept(bytes, offset, length) }
        catch (error: Exception) { failed = true; throw error }
    }
    private fun copy(bytes: ByteArray, offset: Int, length: Int): Int {
        val count = minOf(length, target - used)
        bytes.copyInto(buffer, used, offset, offset + count)
        used += count
        return count
    }
    private fun accept(bytes: ByteArray, offset: Int, length: Int): Feed {
        var consumed = copy(bytes, offset, length)
        if (used == ConnectionManagement.HEADER_BYTES && target == ConnectionManagement.HEADER_BYTES) {
            val header = ConnectionManagement.header(buffer.copyOf(ConnectionManagement.HEADER_BYTES))
            target += header.length
            consumed += copy(bytes, offset + consumed, length - consumed)
        }
        if (used < target) return Feed(consumed, null)
        val message = ConnectionManagement.decode(buffer.copyOf(target))
        buffer.fill(0, 0, target)
        used = 0; target = ConnectionManagement.HEADER_BYTES
        return Feed(consumed, message)
    }
    fun end() {
        check(!ended && !failed)
        ended = true
        if (used != 0) { failed = true; throw IllegalArgumentException("Truncated management record") }
    }
}
