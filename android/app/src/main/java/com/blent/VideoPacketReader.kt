package com.blent

import java.io.EOFException
import java.io.InputStream
import com.blent.VideoWire.FRAME_HEADER_SIZE
import com.blent.VideoWire.MAX_FRAME_SIZE
import com.blent.VideoWire.PACKET_TYPE_CONFIG
import com.blent.VideoWire.PACKET_TYPE_FRAME
import com.blent.VideoWire.PACKET_TYPE_LIVENESS
import com.blent.VideoWire.LIVENESS_VERSION

/** Synchronous consumers must finish using the borrowed bytes before read(). */
internal interface VideoPacketSink {
    fun configuration(data: ByteArray, offset: Int, size: Int)
    fun frame(sequence: Int, data: ByteArray, offset: Int, size: Int)
}

/** Length/type/sequence framing, independent of codec, Surface and session state.
 * Storage belongs to this connection and is reused without per-frame allocation. */
internal class VideoPacketReader(private val input: InputStream, private val livenessEnabled: Boolean = false, private val diagnostic: (String) -> Unit = {}) {
    private val header = ByteArray(4)
    private var data = ByteArray(512 * 1024)
    var size = 0; private set

    fun read(): Boolean {
        if (!readHeader()) return false
        // T587: retain growth headroom only where a future legal packet can use it.
        if (data.size < size) {
            val type = data[0]
            data = ByteArray(minOf(size + size / 2, MAX_FRAME_SIZE + 1))
            data[0] = type
        }
        readExact(data, size, 1)
        return true
    }

    private fun readHeader(): Boolean {
        readExact(header, 4)
        size = bigEndianInt(header, 0)
        if (!validate(size > 1 && size <= MAX_FRAME_SIZE + 1, "Invalid packet size")) return false
        // T1040: consume only the type before admitting payload I/O or growth.
        readExact(data, 1)
        return when (val type = data[0].toInt() and 0xff) {
            PACKET_TYPE_CONFIG -> true
            PACKET_TYPE_FRAME -> validate(size > FRAME_HEADER_SIZE, "Truncated frame packet")
            PACKET_TYPE_LIVENESS -> validate(livenessEnabled && size == 2, "Invalid liveness packet")
            else -> {
                diagnostic("Unknown packet type: $type, reconnecting")
                false
            }
        }
    }

    private fun validate(accepted: Boolean, description: String): Boolean {
        if (!accepted) diagnostic("$description: $size, reconnecting")
        return accepted
    }

    /** Call only after read() succeeds; sinks borrow bytes until the next read. */
    fun dispatch(sink: VideoPacketSink): Boolean {
        when (data[0].toInt() and 0xff) {
            PACKET_TYPE_CONFIG -> sink.configuration(data, 1, size - 1)
            PACKET_TYPE_LIVENESS -> return liveness()
            PACKET_TYPE_FRAME -> {
                sink.frame(bigEndianInt(data, 1), data, FRAME_HEADER_SIZE, size - FRAME_HEADER_SIZE)
            }
            else -> return false
        }
        return true
    }

    // Transport-only progress: no decoder input, connection/frame callback or ACK.
    private fun liveness(): Boolean {
        if (data[1].toInt() != LIVENESS_VERSION) {
            diagnostic("Invalid liveness packet: $size, reconnecting")
            return false
        }
        return true
    }

    private fun readExact(buffer: ByteArray, length: Int, start: Int = 0) {
        var offset = start
        while (offset < length) {
            val read = input.read(buffer, offset, length - offset)
            if (read < 0) throw EOFException("Stream closed")
            offset += read
        }
    }

    private fun bigEndianInt(bytes: ByteArray, offset: Int): Int =
        ((bytes[offset].toInt() and 0xff) shl 24) or
            ((bytes[offset + 1].toInt() and 0xff) shl 16) or
            ((bytes[offset + 2].toInt() and 0xff) shl 8) or
            (bytes[offset + 3].toInt() and 0xff)
}
