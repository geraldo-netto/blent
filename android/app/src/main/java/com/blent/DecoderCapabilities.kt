package com.blent

import android.media.MediaCodecList
import android.media.MediaCodecInfo
import android.media.MediaFormat
import android.os.Build
import org.json.JSONArray
import org.json.JSONObject

/** Bounded query work off Activity/control locks. Snapshots live for one request;
 * advertised support is not a speed measurement. */
internal object DecoderCapabilities {
    private data class Entry(val info: MediaCodecInfo, val types: Array<String>)
    private data class Compatible(val info: MediaCodecInfo, val caps: MediaCodecInfo.CodecCapabilities)

    suspend fun report(width: Int, height: Int, fps: Int): JSONObject =
        report(width, height, fps, ::nativeInventory, ::nativeCapabilities)

    suspend fun report(width: Int, height: Int, fps: Int,
        readInventory: () -> Array<MediaCodecInfo>,
        readCapabilities: (MediaCodecInfo, String) -> MediaCodecInfo.CodecCapabilities,
    ): JSONObject = MetadataQueries.lane.query {
        snapshotReport(width, height, fps, readInventory, readCapabilities)
    } ?: unavailable(width, height, fps)

    private fun unavailable(width: Int, height: Int, fps: Int): JSONObject =
        describe(width, height, fps) { _, _, _, _ -> false }
            .put("hardware", JSONArray()).put("details", JSONArray()).put("status", "unavailable")

    private fun snapshotReport(width: Int, height: Int, fps: Int,
        readInventory: () -> Array<MediaCodecInfo>,
        readCapabilities: (MediaCodecInfo, String) -> MediaCodecInfo.CodecCapabilities,
    ): JSONObject {
        val inventory = snapshot(readInventory)
        val decoders = VideoCodec.types.values.associateWith { mime ->
            compatible(mime, width, height, fps, inventory, readCapabilities)
        }
        return describe(width, height, fps) { mime, _, _, _ -> decoders[mime]?.isNotEmpty() == true }.apply {
            put("hardware", JSONArray(VideoCodec.types.filterValues { mime ->
                decoders[mime]?.any { hardware(it.info) } == true
            }.keys.toList()))
            put("details", details(decoders, width, height, fps))
        }
    }

    private fun details(decoders: Map<String, List<Compatible>>, width: Int, height: Int, fps: Int): JSONArray {
        val rows = JSONArray()
        for ((codec, mime) in VideoCodec.types) {
            val seen = HashSet<String>()
            for (entry in decoders[mime].orEmpty()) {
                val name = entry.info.name
                if (!VideoProfiles.identifier(name) || !seen.add(name)) continue
                val row = MediaInventory.describe(entry.info, codec, width, height, fps, { entry.caps }) ?: continue
                rows.put(row)
                if (rows.length() == 16) return rows
            }
        }
        return rows
    }

    internal fun describe(width: Int, height: Int, fps: Int, supports: (String, Int, Int, Int) -> Boolean): JSONObject {
        val names = VideoCodec.types.filter { (_, mime) -> supports(mime, width, height, fps) }.keys
        return JSONObject().apply {
            put("protocol", 2); put("width", width); put("height", height); put("fps", fps)
            put("codecs", JSONArray(names.toList()))
        }
    }
    internal fun hardware(info: MediaCodecInfo): Boolean =
        Build.VERSION.SDK_INT >= 29 && info.isHardwareAccelerated

    private fun snapshot(read: () -> Array<MediaCodecInfo>): List<Entry> = try {
        read().mapNotNull { info ->
            try {
                if (info.isEncoder) null else Entry(info, info.supportedTypes)
            } catch (_: Exception) { null }
        }
    } catch (_: Exception) { emptyList() }

    private fun compatible(mime: String, width: Int, height: Int, fps: Int, inventory: List<Entry>,
        readCapabilities: (MediaCodecInfo, String) -> MediaCodecInfo.CodecCapabilities,
    ): List<Compatible> = try {
        val format = MediaFormat.createVideoFormat(mime, width, height)
        format.setInteger(MediaFormat.KEY_FRAME_RATE, fps)
        matching(format, mime, inventory, readCapabilities)
    } catch (_: Exception) { emptyList() }

    internal fun decoderName(format: MediaFormat, mime: String): String? {
        val choices = matching(format, mime, snapshot(::nativeInventory), ::nativeCapabilities).map { it.info }
        return (choices.firstOrNull(::hardware) ?: choices.firstOrNull())?.name
    }
    private fun nativeCapabilities(info: MediaCodecInfo, mime: String) = info.getCapabilitiesForType(mime)
    private fun nativeInventory(): Array<MediaCodecInfo> = MediaCodecList(MediaCodecList.REGULAR_CODECS).codecInfos

    private fun matching(format: MediaFormat, mime: String, inventory: List<Entry>,
        readCapabilities: (MediaCodecInfo, String) -> MediaCodecInfo.CodecCapabilities,
    ): List<Compatible> = inventory.mapNotNull { entry ->
        if (entry.types.none { it.equals(mime, ignoreCase = true) }) return@mapNotNull null
        try {
            val caps = readCapabilities(entry.info, mime)
            if (caps.isFormatSupported(format)) Compatible(entry.info, caps) else null
        } catch (_: Exception) { null }
    }
}
