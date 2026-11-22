package com.blent

import android.media.MediaCodec
import android.media.MediaFormat
import android.os.Build

internal typealias DecoderFormatFactory = (MediaCodec, DecoderFormat, Boolean) -> MediaFormat

internal object DecoderConfiguration {
    fun create(parameters: DecoderFormat): MediaCodec {
        val format = MediaFormat.createVideoFormat(parameters.mimeType, parameters.width, parameters.height)
        format.setInteger(MediaFormat.KEY_FRAME_RATE, parameters.fps)
        val name = checkNotNull(parameters.selection?.validate(parameters) ?: DecoderCapabilities.decoderName(format, parameters.mimeType)) {
            "No compatible decoder for stream format"
        }
        return try { MediaCodec.createByCodecName(name) }
        catch (failure: Exception) { throw IllegalStateException("Unable to create compatible decoder $name", failure) }
    }

    fun format(parameters: DecoderFormat, allowHints: Boolean): MediaFormat {
        val format = baseFormat(parameters, allowHints)
        if (allowHints && parameters.selection == null) defaultHints(parameters, format)
        return format
    }

    fun baseFormat(parameters: DecoderFormat, allowHints: Boolean): MediaFormat {
        val format = MediaFormat.createVideoFormat(parameters.mimeType, parameters.width, parameters.height)
        parameters.codecPrivate?.takeIf { it.isNotEmpty() }?.let {
            format.setByteBuffer("csd-0", java.nio.ByteBuffer.wrap(it))
        }
        format.setInteger(MediaFormat.KEY_FRAME_RATE, parameters.fps)
        format.setInteger(MediaFormat.KEY_I_FRAME_INTERVAL, 1)
        format.setInteger(MediaFormat.KEY_COLOR_RANGE, MediaFormat.COLOR_RANGE_LIMITED)
        format.setInteger(MediaFormat.KEY_COLOR_STANDARD, MediaFormat.COLOR_STANDARD_BT709)
        format.setInteger(MediaFormat.KEY_COLOR_TRANSFER, MediaFormat.COLOR_TRANSFER_SDR_VIDEO)
        if (parameters.selection != null) parameters.selection.configure(format, allowHints)
        return format
    }

    private fun defaultHints(parameters: DecoderFormat, format: MediaFormat) {
        if (Build.VERSION.SDK_INT >= 30) format.setInteger(MediaFormat.KEY_LOW_LATENCY, 1)
        format.setInteger(MediaFormat.KEY_OPERATING_RATE, 2 * parameters.fps)
        format.setInteger("vendor.qti-ext-dec-low-latency.enable", 1)
    }
}
