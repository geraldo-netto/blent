package com.blent

import android.media.MediaCodec
import java.nio.ByteBuffer

internal fun queueCodecInput(codec: MediaCodec, index: Int, info: DecoderInputInfo, fill: (ByteBuffer) -> Unit) {
    val buffer = checkNotNull(codec.getInputBuffer(index)) { "Decoder returned no input buffer" }
    require(info.size in 1..buffer.capacity()) { "Access unit exceeds codec input capacity" }
    buffer.clear().limit(info.size)
    fill(buffer)
    check(buffer.position() == info.size) { "Incomplete codec input payload" }
    val flags = if (info.configuration) MediaCodec.BUFFER_FLAG_CODEC_CONFIG else 0
    codec.queueInputBuffer(index, 0, info.size, info.sequence, flags)
}

/** Copy a packet into an admitted native buffer without retaining its view. */
internal fun DecoderInput.copyTo(buffer: ByteBuffer) { buffer.put(data, offset, size) }
