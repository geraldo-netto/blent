package com.blent

import android.media.MediaCodecList
import android.media.MediaCodecInfo
import android.media.MediaFormat
import android.os.Build
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import org.json.JSONArray
import org.json.JSONObject

/** Serialized off the Activity/control locks. Advertised support is not a speed measurement. */
internal object DecoderCapabilities {
    private val admission = Mutex()
    suspend fun report(width: Int, height: Int, fps: Int): JSONObject = admission.withLock {
        val decoders = VideoCodec.types.values.associateWith { mime -> compatible(mime, width, height, fps) }
        describe(width, height, fps) { mime, _, _, _ ->
            decoders[mime]?.isNotEmpty() == true
        }.apply {
            put("hardware", JSONArray(VideoCodec.types.filterValues { mime -> decoders[mime]?.any(::hardware) == true }.keys.toList()))
            val entries = VideoCodec.types.flatMap { (codec, mime) ->
                decoders[mime].orEmpty().mapNotNull { MediaInventory.describe(it, codec, width, height, fps) }
            }.distinctBy { it.getString("codec") to it.getString("name") }.take(16)
            put("details", JSONArray(entries))
        }
    }
    internal fun describe(width: Int, height: Int, fps: Int, supports: (String, Int, Int, Int) -> Boolean): JSONObject {
        val names = VideoCodec.types.filter { (_, mime) -> supports(mime, width, height, fps) }.keys
        return JSONObject().apply {
            put("protocol", 1); put("width", width); put("height", height); put("fps", fps)
            put("codecs", JSONArray(names.toList()))
        }
    }
    internal fun hardware(info: MediaCodecInfo): Boolean =
        Build.VERSION.SDK_INT >= 29 && info.isHardwareAccelerated

    private fun compatible(mime: String, width: Int, height: Int, fps: Int): List<MediaCodecInfo> = try {
        val format = MediaFormat.createVideoFormat(mime, width, height)
        format.setInteger(MediaFormat.KEY_FRAME_RATE, fps)
        compatible(format, mime)
    } catch (_: Exception) { emptyList() }

    internal fun decoderName(format: MediaFormat, mime: String): String? {
        val choices = compatible(format, mime)
        return (choices.firstOrNull(::hardware) ?: choices.firstOrNull())?.name
    }
    private fun compatible(format: MediaFormat, mime: String): List<MediaCodecInfo> =
        MediaCodecList(MediaCodecList.REGULAR_CODECS).codecInfos.filter { info ->
            try { !info.isEncoder && info.getCapabilitiesForType(mime).isFormatSupported(format) }
            catch (_: Exception) { false }
        }
}
