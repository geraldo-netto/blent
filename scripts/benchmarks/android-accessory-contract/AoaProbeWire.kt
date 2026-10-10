package com.blent

import java.io.InputStream
import java.io.OutputStream
import java.nio.ByteBuffer
import java.nio.ByteOrder
import java.util.zip.CRC32

/** T846 synthetic integrity probe only. This protocol grants no app authority. */
internal object AoaProbeWire {
    const val CAPACITY = 65536
    const val HEADER_BYTES = 24
    const val MAX_PACKETS = 4096
    const val MAGIC = 0x42414f31
    data class Header(val session: Long, val sequence: Int, val length: Int, val checksum: Int)

    fun decode(bytes: ByteArray): Header {
        require(bytes.size == HEADER_BYTES)
        val buffer = ByteBuffer.wrap(bytes).order(ByteOrder.BIG_ENDIAN)
        require(buffer.int == MAGIC) { "Invalid probe magic" }
        val header = Header(buffer.long, buffer.int, buffer.int, buffer.int)
        require(header.session != 0L) { "Missing probe session" }
        require(header.sequence in 0 until MAX_PACKETS) { "Invalid probe sequence" }
        require(header.length in 1..CAPACITY) { "Invalid probe length" }
        return header
    }

    fun readFully(input: InputStream, bytes: ByteArray, length: Int) {
        require(length in 0..bytes.size)
        var offset = 0
        while (offset < length) {
            val count = input.read(bytes, offset, length - offset)
            check(count in 1..(length - offset)) { "Probe stream ended or made no progress" }
            offset += count
        }
    }

    fun checksum(bytes: ByteArray, length: Int): Int {
        require(length in 0..bytes.size)
        return CRC32().apply { update(bytes, 0, length) }.value.toInt()
    }

    fun echo(input: InputStream, output: OutputStream, report: (Int, Long) -> Unit): Long {
        val raw = ByteArray(HEADER_BYTES)
        val payload = ByteArray(CAPACITY)
        var session = 0L
        var total = 0L
        repeat(MAX_PACKETS) { sequence ->
            readFully(input, raw, raw.size)
            val header = decode(raw)
            if (sequence == 0) session = header.session
            check(header.session == session && header.sequence == sequence) { "Stale probe frame" }
            readFully(input, payload, header.length)
            check(checksum(payload, header.length) == header.checksum) { "Probe checksum mismatch" }
            output.write(raw)
            output.write(payload, 0, header.length)
            total += header.length
            if (sequence % 64 == 0) report(sequence + 1, total)
        }
        return total
    }
}
