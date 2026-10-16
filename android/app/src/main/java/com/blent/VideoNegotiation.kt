package com.blent

import org.json.JSONObject

/** Optional capability and acknowledgement, independent of media/native ownership. */
internal object VideoNegotiation {
    fun offered(message: JSONObject): Boolean {
        if (message.optString("status") != "connected" || !message.has("video_liveness")) return false
        return runCatching { JsonNumbers.integer(message, "video_liveness", 1, 1) == 1 }.getOrDefault(false)
    }
    fun acknowledgement(): ByteArray = "!BLENT1\n".toByteArray(Charsets.US_ASCII)
}
