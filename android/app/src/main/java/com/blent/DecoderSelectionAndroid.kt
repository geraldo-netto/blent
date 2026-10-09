package com.blent

import android.media.MediaCodecList
import android.media.MediaFormat
import org.json.JSONObject

/** Native capability and configuration adapter for an independent selection value. */
internal fun DecoderSelection.validate(parameters: DecoderFormat): String {
    require(VideoCodec.types[codec] == parameters.mimeType) { "Decoder selection codec mismatch" }
    val info = MediaCodecList(MediaCodecList.REGULAR_CODECS).codecInfos.firstOrNull { !it.isEncoder && it.name == name }
    requireNotNull(info) { "Selected decoder is unavailable" }
    val report = MediaInventory.describe(info, codec, parameters.width, parameters.height, parameters.fps)
    requireNotNull(report) { "Selected decoder inventory unavailable" }
    val profiles = report.getJSONArray("profiles")
    require((0 until profiles.length()).any { MediaProfiles.covers(profiles.getJSONObject(it), stream()) }) {
        "Selected decoder does not support stream profile/level/depth"
    }
    validateHints(report)
    return name
}

private fun DecoderSelection.validateHints(report: JSONObject) {
    require(!lowLatency || report.optBoolean("low_latency")) { "Unsupported low latency hint" }
    require(operatingRate == null || report.optInt("operating_rate") >= operatingRate) { "Unsupported operating rate hint" }
}

internal fun DecoderSelection.configure(format: MediaFormat, allowHints: Boolean) {
    format.setInteger(MediaFormat.KEY_PROFILE, checkNotNull(MediaProfiles.profile(codec, profile, depth)).android)
    if (!allowHints) return
    if (lowLatency && android.os.Build.VERSION.SDK_INT >= 30) format.setInteger(MediaFormat.KEY_LOW_LATENCY, 1)
    operatingRate?.let { format.setInteger(MediaFormat.KEY_OPERATING_RATE, it) }
}
