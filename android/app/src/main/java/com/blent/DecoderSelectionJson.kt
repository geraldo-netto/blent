package com.blent

import org.json.JSONObject

internal object DecoderSelectionJson {
    fun read(message: JSONObject): DecoderSelection? {
        if (message.isNull("decoder_selection")) return null
        JsonNumbers.integer(message, "decoder_protocol", 2, 2)
        val selection = message.getJSONObject("decoder_selection")
        val stream = selection.getJSONObject("stream")
        val name = selection.getString("name")
        require(VideoProfiles.identifier(name)) { "Invalid decoder identity" }
        val codec = stream.getString("codec")
        val profile = stream.getString("profile")
        val depth = JsonNumbers.integer(stream, "depth", 8, 10)
        require(VideoProfiles.profile(codec, profile, depth)) { "Unknown stream profile" }
        val level = JsonNumbers.integer(stream, "level", 9, 73)
        require(VideoProfiles.validLevel(codec, level)) { "Invalid stream level" }
        val rate = if (selection.isNull("operating_rate")) null else
            JsonNumbers.integer(selection, "operating_rate", 10, 180)
        return DecoderSelection(name, codec, profile, level, depth, selection.getBoolean("low_latency"), rate)
    }
}

internal fun DecoderSelection.stream(): JSONObject = JSONObject().put("codec", codec).put("profile", profile).put("level", level).put("depth", depth)
