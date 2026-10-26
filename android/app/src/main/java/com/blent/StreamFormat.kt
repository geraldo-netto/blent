package com.blent

import org.json.JSONObject

/** One validated control format for decoder setup and capability queries. */
internal object StreamFormat {
    fun read(message: JSONObject, previous: DecoderFormat?, defaultFps: Int): DecoderFormat? = try {
        val mime = if (message.has("codec")) VideoCodec.types[message.getString("codec")]
            else previous?.mimeType
        requireNotNull(mime) { "Unsupported host codec" }
        val (width, height) = dimensions(message, previous)
        val fps = JsonNumbers.optional(message, "fps", previous?.fps ?: defaultFps, 10, 90)
        require(fps in 10..90) { "Invalid stream FPS" }
        val selection = DecoderSelectionJson.read(message)
        require(selection == null || VideoCodec.types[selection.codec] == mime) { "Decoder selection codec mismatch" }
        DecoderFormat(mime, width, height, fps, selection = selection)
    } catch (_: Exception) { null }

    private fun dimensions(message: JSONObject, previous: DecoderFormat?): Pair<Int, Int> {
        // Partial updates may retain only an already validated current format.
        val pixels = if (message.has("video_width") || message.has("video_height")) {
            JsonNumbers.integer(message, "video_width", 2, 4096) to
                JsonNumbers.integer(message, "video_height", 2, 4096)
        } else if (previous != null) {
            previous.width to previous.height
        } else {
            error("Missing initial encoded dimensions")
        }
        require(pixels.first in 2..4096 && pixels.second in 2..4096) { "Invalid stream dimensions" }
        return pixels
    }
}
