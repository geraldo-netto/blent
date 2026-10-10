package com.blent

import org.json.JSONArray
import org.json.JSONObject

internal object WorkerEventJson {
    const val LIMIT = 256 * 1024
    fun encode(event: WorkerEvent): String = when (event) {
        is WorkerEvent.Status -> JSONObject().put("type", "status").put("text", event.text.take(2048))
        is WorkerEvent.Result -> JSONObject().put("type", "result").put("text", event.text)
        is WorkerEvent.Ended -> JSONObject().put("type", "ended").put("failure", event.failure?.take(2048))
        is WorkerEvent.Rendered -> JSONObject().put("type", "rendered").put("sequence", event.sequence).put("decode", event.decodeMicros).put("decoder", event.decoder)
        is WorkerEvent.Display -> display(event)
    }.toString().also { require(it.length <= LIMIT) }

    fun decode(text: String): WorkerEvent {
        require(text.length <= LIMIT)
        val v = WorkerJsonBoundary.parse(text, LIMIT)
        return when (v.getString("type")) {
            "status" -> WorkerEvent.Status(v.getString("text"))
            "result" -> WorkerEvent.Result(v.getString("text"))
            "ended" -> WorkerEvent.Ended(optional(v, "failure"))
            "rendered" -> WorkerEvent.Rendered(v.getInt("sequence"), v.getInt("decode"), optional(v, "decoder"))
            "display" -> WorkerEvent.Display(v.getBoolean("connected"), v.getDouble("fps").toFloat(), v.getDouble("mbps").toFloat(),
                optional(v, "startup"), optional(v, "retirement"), DecoderDiagnostics(readActive(v.optJSONObject("active")), v.getBoolean("fallback")))
            else -> error("Unknown media worker event")
        }
    }
    private fun display(v: WorkerEvent.Display) = JSONObject().put("type", "display").put("connected", v.connected)
        .put("fps", v.fps).put("mbps", v.mbps).put("startup", v.startup).put("retirement", v.retirement)
        .put("fallback", v.diagnostics.watchdogFallback).put("active", active(v.diagnostics.active))

    private fun active(v: ActiveDecoderDiagnostics?): JSONObject? = v?.let {
        JSONObject().put("name", it.name).put("mime", it.mime).put("width", it.width).put("height", it.height).put("fps", it.fps)
            .put("hardware", it.hardware).put("low", it.lowLatency).put("double", it.doubleRate).put("requested", JSONArray(it.requested))
    }
    private fun readActive(v: JSONObject?): ActiveDecoderDiagnostics? = v?.let {
        val requested = it.getJSONArray("requested")
        require(requested.length() <= 16)
        ActiveDecoderDiagnostics(optional(it, "name"), it.getString("mime"), it.getInt("width"), it.getInt("height"), it.getInt("fps"),
            optionalBoolean(it, "hardware"), optionalBoolean(it, "low"), optionalBoolean(it, "double"), List(requested.length()) { index -> requested.getString(index) })
    }
    private fun optional(v: JSONObject, name: String) = if (v.isNull(name)) null else v.getString(name)
    private fun optionalBoolean(v: JSONObject, name: String) = if (v.isNull(name)) null else v.getBoolean(name)
}
