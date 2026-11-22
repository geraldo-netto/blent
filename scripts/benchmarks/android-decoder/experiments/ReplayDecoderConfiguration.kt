package com.blent

import android.media.MediaCodec
import android.media.MediaCodecInfo
import android.media.MediaFormat
import android.os.Build

/** Replay-only alternatives. Production defaults and negotiated format selection
 * stay authoritative in DecoderConfiguration; this adapter borrows one codec. */
internal object ReplayDecoderConfiguration {
    interface Support {
        fun lowLatency(codec: MediaCodec, mime: String): Boolean
        fun operatingRate(codec: MediaCodec, parameters: DecoderFormat, fps: Int): Boolean
    }

    fun format(codec: MediaCodec, parameters: DecoderFormat, allowHints: Boolean,
               multiplier: Int? = 2, support: Support = PlatformSupport): MediaFormat {
        val format = DecoderConfiguration.baseFormat(parameters, allowHints)
        if (allowHints && parameters.selection == null) supportedHints(codec, parameters, format, multiplier, support)
        return format
    }

    private fun supportedHints(codec: MediaCodec, parameters: DecoderFormat, format: MediaFormat,
                               multiplier: Int?, support: Support) {
        if (Build.VERSION.SDK_INT >= 30 && support.lowLatency(codec, parameters.mimeType)) {
            format.setInteger(MediaFormat.KEY_LOW_LATENCY, 1)
        }
        val rate = multiplier?.times(parameters.fps)
        if (rate != null && support.operatingRate(codec, parameters, rate)) {
            format.setInteger(MediaFormat.KEY_OPERATING_RATE, rate)
        }
    }

    private object PlatformSupport : Support {
        override fun lowLatency(codec: MediaCodec, mime: String): Boolean = try {
            Build.VERSION.SDK_INT >= 30 && codec.codecInfo.getCapabilitiesForType(mime)
                .isFeatureSupported(MediaCodecInfo.CodecCapabilities.FEATURE_LowLatency)
        } catch (_: Exception) { false }

        override fun operatingRate(codec: MediaCodec, parameters: DecoderFormat, fps: Int): Boolean = try {
            codec.codecInfo.getCapabilitiesForType(parameters.mimeType).videoCapabilities
                ?.areSizeAndRateSupported(parameters.width, parameters.height, fps.toDouble()) == true
        } catch (_: Exception) { false }
    }
}
