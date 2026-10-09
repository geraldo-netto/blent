package com.blent

import java.nio.ByteBuffer
import java.nio.ByteOrder

/** Wire identity and framed codec configuration; unrelated to the host muxer. */
internal object VideoCodec {
    val types = mapOf("h264" to "video/avc", "hevc" to "video/hevc", "vp9" to "video/x-vnd.on2.vp9", "av1" to "video/av01")
    fun hasConfigurationHeader(data: ByteArray, offset: Int, size: Int): Boolean =
        size >= 4 && ByteBuffer.wrap(data, offset, 4).int == 0x424c4e31
    private val framedTypes = mapOf(3 to types.getValue("vp9"), 4 to types.getValue("av1"))
    fun framed(mime: String) = mime in framedTypes.values

    fun configuration(data: ByteArray, offset: Int, size: Int, mime: String, fps: Int): DecoderFormat {
        require(size in 13..65549) { "Invalid framed codec configuration length" }
        val bytes = ByteBuffer.wrap(data, offset, size).order(ByteOrder.BIG_ENDIAN)
        require(bytes.int == 0x424c4e31) { "Unsupported video configuration version" }
        require(framedTypes[bytes.get().toInt()] == mime) { "Video/control codec mismatch" }
        val width = bytes.int
        val height = bytes.int
        require(width in 2..4096 && height in 2..4096) { "Invalid stream dimensions" }
        val private = ByteArray(bytes.remaining()).also { bytes.get(it) }
        return DecoderFormat(mime, width, height, fps, private)
    }
}
