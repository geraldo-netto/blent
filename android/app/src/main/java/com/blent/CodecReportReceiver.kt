package com.blent

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.media.MediaCodecInfo
import android.media.MediaCodecList
import android.os.Build
import kotlinx.coroutines.*

/** Read-only decoder inventory for `adb shell am broadcast`; DUMP restricts callers to shell/system. */
class CodecReportReceiver(private val query: (Int) -> String = { decoderReport(it) }) : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
        val version = intent.getIntExtra("blent_codecs_version", 1)
        val completion = goAsync()
        CoroutineScope(Dispatchers.Default).launch {
            try {
                completion.resultData = MetadataQueries.lane.query { query(version) }
                    ?: formatDecoderReport(version, null)
            } finally { completion.finish() }
        }
    }
}

internal fun decoderReport(
    version: Int = 1,
    codecs: () -> Array<MediaCodecInfo> = { MediaCodecList(MediaCodecList.REGULAR_CODECS).codecInfos },
): String {
    val inventory = try { codecs() } catch (_: Exception) { null }
    return formatDecoderReport(version, inventory)
}

internal fun formatDecoderReport(version: Int, inventory: Array<MediaCodecInfo>?): String {
    if (version == 2) {
        return "BLENT_CODECS_V2:" + VideoCodec.types.entries.joinToString(";") { (name, mime) ->
            "$name=${decoderInventory(inventory, mime)}"
        }
    }
    // Preserve the shell API used by older hosts: V1 describes HEVC only.
    return "BLENT_CODECS_V1:" + decoderInventory(inventory, "video/hevc")
}

private fun decoderInventory(codecs: Array<MediaCodecInfo>?, mime: String): String {
    if (codecs == null) return "unknown"
    return try {
        val entries = codecs.filter { !it.isEncoder && mime in it.supportedTypes }
            .mapNotNull { decoderEntry(it, mime) }.distinct().sorted()
        if (entries.isEmpty()) "none" else entries.joinToString(",")
    } catch (_: Exception) { "unknown" }
}

private fun decoderEntry(codec: MediaCodecInfo, mime: String): String? {
    val capabilities = codec.getCapabilitiesForType(mime)
    if (capabilities.isFeatureRequired(MediaCodecInfo.CodecCapabilities.FEATURE_SecurePlayback) ||
        capabilities.isFeatureRequired(MediaCodecInfo.CodecCapabilities.FEATURE_TunneledPlayback)) {
        return null
    }
    val acceleration = decoderAcceleration(codec)
    return if (mime == "video/hevc") acceleration + hevcDepth(capabilities) else if (acceleration == "unknown") "unclassified" else acceleration
}

private fun hevcDepth(capabilities: MediaCodecInfo.CodecCapabilities): String {
    val main10 = capabilities.profileLevels.any {
        it.profile == MediaCodecInfo.CodecProfileLevel.HEVCProfileMain10 ||
            it.profile == MediaCodecInfo.CodecProfileLevel.HEVCProfileMain10HDR10 ||
            it.profile == MediaCodecInfo.CodecProfileLevel.HEVCProfileMain10HDR10Plus
    }
    return if (main10) "10" else "8"
}

private fun decoderAcceleration(codec: MediaCodecInfo): String = when {
    Build.VERSION.SDK_INT >= 29 && codec.isHardwareAccelerated -> "hw"
    Build.VERSION.SDK_INT >= 29 && codec.isSoftwareOnly -> "sw"
    codec.name.startsWith("OMX.google.") || codec.name.startsWith("c2.android.") -> "sw"
    else -> "unknown"
}
