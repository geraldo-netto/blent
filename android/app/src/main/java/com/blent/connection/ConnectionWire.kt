package com.blent.connection

import java.nio.ByteBuffer
import java.nio.ByteOrder

/** T846 portable envelope. Parsing grants no peer identity or media consent. */
internal object ConnectionWire {
    const val HEADER_BYTES = 40
    const val CHANNELS = 6
    const val MAX_PAYLOAD = 65536
    const val MAX_CREDIT = 1048576
    const val DATA = 0
    const val CREDIT = 1
    const val CLOSE = 2
    const val RESET = 3
    const val CONTROL = 1
    const val DISPLAY = 2
    const val CAMERA = 3
    const val MICROPHONE = 4
    const val SPEAKER = 5
    const val MANAGEMENT = 6

    data class Header(
        val kind: Int, val channel: Int, val session: Long, val generation: Long,
        val sequence: Long, val length: Int, val credit: Int,
    )

    fun validate(header: Header) {
        require(header.kind in DATA..RESET && header.channel in CONTROL..MANAGEMENT)
        require(header.session > 0 && header.generation > 0 && header.sequence >= 0)
        validateBody(header)
    }

    private fun validateBody(header: Header) {
        when (header.kind) {
            DATA -> require(header.length in 1..MAX_PAYLOAD && header.credit == 0)
            CREDIT -> require(header.length == 0 && header.credit in 1..MAX_CREDIT)
            else -> require(header.length == 0 && header.credit == 0)
        }
    }

    /** Validate before changing the output; callers can reuse the fixed header. */
    fun encode(header: Header, output: ByteArray) {
        require(output.size == HEADER_BYTES)
        validate(header)
        ByteBuffer.wrap(output).order(ByteOrder.BIG_ENDIAN).apply {
            putInt(0x424c4e54); put(1); put(header.kind.toByte()); put(header.channel.toByte()); put(0)
            putLong(header.session); putLong(header.generation); putLong(header.sequence)
            putInt(header.length); putInt(header.credit)
        }
    }

    fun decode(input: ByteArray): Header {
        require(input.size == HEADER_BYTES)
        val buffer = ByteBuffer.wrap(input).order(ByteOrder.BIG_ENDIAN)
        require(buffer.int == 0x424c4e54 && buffer.get().toInt() == 1)
        val kind = buffer.get().toInt()
        val channel = buffer.get().toInt()
        require(buffer.get().toInt() == 0)
        return Header(kind, channel, buffer.long, buffer.long, buffer.long, buffer.int, buffer.int)
            .also(::validate)
    }
}
