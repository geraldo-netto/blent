package com.blent

import com.blent.connection.*
import java.io.DataInputStream
import java.io.OutputStream

/** Synthetic qualification only; no media or application authority. Caller owns
 * a bounded socket read deadline and supervised native process lifetime. */
internal class AoaSecureEcho(
    private val frames: Int = 4096,
    private val stopped: () -> Boolean = { false },
    private val report: (String) -> Unit = {},
) {
    init { require(frames in 1..4096) }
    fun exchange(source: java.io.InputStream, output: OutputStream): Long {
        val mux = ConnectionMux(7, 9, 1048576, 64, 65536).also { owner ->
            for (channel in 1 until ConnectionWire.CHANNELS) owner.activate(channel, 9)
        }
        val input = DataInputStream(source)
        var bytes = 0L
        var sequence = 0
        flush(output, mux)
        while (sequence < frames) {
            check(!stopped())
            val header = ConnectionWire.decode(ByteArray(ConnectionWire.HEADER_BYTES).also(input::readFully))
            val deliver = mux.receive(header)
            val body = ByteArray(header.length).also(input::readFully)
            if (header.kind != ConnectionWire.DATA) continue
            check(deliver)
            validate(sequence, header, body)
            check(mux.offer(header.channel, body))
            mux.release(header.channel, header.length)
            flush(output, mux)
            bytes += body.size
            sequence++
            if (sequence % 256 == 0) report("Encrypted USB: $sequence frames; $bytes bytes echoed.")
        }
        receipt(input, mux)
        return bytes
    }

    private fun receipt(input: DataInputStream, mux: ConnectionMux) {
        val expected = "BLENTUSB-FINISH".toByteArray(Charsets.US_ASCII) + byteArrayOf(1)
        repeat(ConnectionWire.CHANNELS + 1) {
            val header = ConnectionWire.decode(ByteArray(ConnectionWire.HEADER_BYTES).also(input::readFully))
            val deliver = mux.receive(header)
            if (header.kind == ConnectionWire.CREDIT) return@repeat
            check(deliver && header.channel == ConnectionWire.MANAGEMENT && header.length == expected.size)
            val body = ByteArray(header.length).also(input::readFully)
            check(body.contentEquals(expected)) { "Invalid host completion receipt" }
            return
        }
        error("Host completion receipt exceeded metadata bound")
    }

    private fun validate(sequence: Int, header: ConnectionWire.Header, body: ByteArray) {
        check(header.channel == sequence % ConnectionWire.CHANNELS + 1)
        check(body.size == intArrayOf(1, 64, 512, 4096, 16384, 65536)[(sequence / ConnectionWire.CHANNELS) % 6])
        for (index in body.indices) check(body[index] == (index * 31 + sequence * 17).toByte()) { "Payload mismatch" }
    }

    private fun flush(output: OutputStream, mux: ConnectionMux) {
        while (true) {
            val frame = mux.next() ?: break
            val header = ByteArray(ConnectionWire.HEADER_BYTES)
            ConnectionWire.encode(frame.header, header)
            output.write(header); output.write(frame.bytes, frame.offset, frame.length); output.flush()
            mux.finish(true)
        }
    }
}
