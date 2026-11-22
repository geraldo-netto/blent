package com.blent

import org.json.JSONObject

internal object StreamOverridesJson {
    fun read(message: JSONObject): StreamOverrides? = try {
        StreamOverrides(optional(message, "bitrate", 1000, 60000), optional(message, "fps", 10, 90))
    } catch (_: Exception) { null }

    private fun optional(message: JSONObject, name: String, minimum: Int, maximum: Int): Int? =
        if (message.isNull(name)) null else JsonNumbers.integer(message, name, minimum, maximum)

    fun config(values: StreamOverrides): JSONObject {
        require(!values.empty)
        return JSONObject().apply {
            put("type", "config")
            values.bitrateKbps?.let { put("bitrate", it) }
            values.fps?.let { put("fps", it) }
        }
    }
}
