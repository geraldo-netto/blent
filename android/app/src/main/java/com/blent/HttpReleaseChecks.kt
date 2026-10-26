package com.blent

import android.util.Log
import okhttp3.Call
import okhttp3.Callback
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.Response
import okhttp3.ResponseBody.Companion.toResponseBody
import android.util.JsonReader
import android.util.JsonToken
import java.io.StringReader
import java.io.IOException
import java.util.concurrent.TimeUnit

internal class HttpReleaseChecks(
    private val endpoint: String,
    client: OkHttpClient = OkHttpClient(),
    deadlineMillis: Long = 15_000,
    private val maxResponseBytes: Int = DEFAULT_RESPONSE_BYTES,
) : ReleaseChecks {
    companion object {
        const val DEFAULT_RESPONSE_BYTES = 256 * 1024
        const val MAX_RESPONSE_BYTES = 1024 * 1024
    }
    init { require(maxResponseBytes in 1..MAX_RESPONSE_BYTES) }
    private val client = client.newBuilder()
        .connectTimeout(10, TimeUnit.SECONDS)
        .readTimeout(10, TimeUnit.SECONDS)
        .callTimeout(deadlineMillis, TimeUnit.MILLISECONDS)
        .build()

    override fun newCall(current: String): ReleaseCall {
        val request = Request.Builder().url(endpoint)
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", "blent-android/$current").build()
        return HttpReleaseCall(client.newCall(request), current, maxResponseBytes)
    }
}

private class HttpReleaseCall(
    private val call: Call, private val current: String, private val maxResponseBytes: Int,
) : ReleaseCall {
    override fun cancel() = call.cancel()
    override fun start(result: (String?) -> Unit) {
        call.enqueue(object : Callback {
            override fun onFailure(call: Call, e: IOException) { result(null) }
            override fun onResponse(call: Call, response: Response) {
                val found = try { response.use { newer(it) } } catch (error: Exception) {
                    Log.d("BlentUpdate", "update check skipped: ${error.message}")
                    null
                }
                result(found)
            }
        })
    }
    private fun newer(response: Response): String? {
        if (!response.isSuccessful) return null
        val tag = releaseTag(boundedBody(response) ?: return null) ?: return null
        return if (UpdateCheck.isNewer(tag, current)) tag.trim().removePrefix("v") else null
    }
    // The response callback owns the original source until response.use closes
    // it. Admit at most the configured bytes before allocating decoded text.
    // Okio may retain one additional 8KiB upstream segment while rejecting.
    private fun boundedBody(response: Response): String? {
        val body = response.body ?: return null
        val length = body.contentLength()
        if (length < -1 || length > maxResponseBytes) return null
        val source = body.source()
        if (source.request(maxResponseBytes.toLong() + 1)) return null
        if (length >= 0 && source.buffer.size != length) return null
        // Only the bounded, detached bytes reach the ordinary BOM/charset
        // decoder. Its separate in-memory body owns no network resource.
        return source.buffer.readByteArray().toResponseBody(body.contentType()).string()
    }

}

/** Parse a complete strict JSON object; unknown fields do not change the tag. */
private fun releaseTag(text: String): String? = JsonReader(StringReader(text)).use { reader ->
    reader.beginObject()
    var tag: String? = null
    while (reader.hasNext()) {
        if (reader.nextName() == "tag_name") tag = stringValue(reader)
        else reader.skipValue()
    }
    reader.endObject()
    require(reader.peek() == JsonToken.END_DOCUMENT) { "Trailing release metadata" }
    tag
}

private fun stringValue(reader: JsonReader): String? {
    if (reader.peek() == JsonToken.STRING) return reader.nextString()
    reader.skipValue()
    return null
}
