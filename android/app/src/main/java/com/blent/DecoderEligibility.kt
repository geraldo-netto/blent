package com.blent

import android.media.MediaCodecInfo

/** Shared metadata policy for ordinary surface playback. Each query borrows one
 * inventory snapshot; matching never creates or owns a native decoder. */
internal object DecoderEligibility {
    fun type(advertised: Array<String>, requested: String): String? =
        advertised.firstOrNull { it.equals(requested, ignoreCase = true) }

    fun usable(capabilities: MediaCodecInfo.CodecCapabilities): Boolean =
        !capabilities.isFeatureRequired(MediaCodecInfo.CodecCapabilities.FEATURE_SecurePlayback) &&
            !capabilities.isFeatureRequired(MediaCodecInfo.CodecCapabilities.FEATURE_TunneledPlayback)
}
