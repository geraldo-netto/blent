package com.blent

import org.json.JSONObject

/** Current paired wire values only; partial or invalid observations are unavailable. */
internal object HostStreamSettingsJson {
    fun read(message: JSONObject): HostStreamSettings? = try {
        HostStreamSettings(
            JsonNumbers.integer(message, "bitrate", 1000, 60000),
            JsonNumbers.integer(message, "fps", 10, 90),
        )
    } catch (_: Exception) { null }
}
