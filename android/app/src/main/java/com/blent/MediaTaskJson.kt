package com.blent

import org.json.JSONObject

/** Versioned IPC adapter. No credentials or payloads are logged. */
internal object MediaTaskJson {
    const val LIMIT = 32 * 1024
    fun encode(task: MediaTask): String = when (task) {
        is MediaTask.Audio -> audio(task)
        is MediaTask.Camera -> camera(task)
        is MediaTask.Display -> display(task)
        is MediaTask.Wake -> WakeTaskJson.encode(task)
        is MediaTask.Metadata -> JSONObject().put("kind", "metadata").put("width", task.width).put("height", task.height)
            .put("fps", task.fps).put("shell", task.shellVersion)
    }.put("version", 1).toString().also { require(it.length <= LIMIT) }

    fun decode(text: String): MediaTask {
        require(text.length <= LIMIT)
        val value = WorkerJsonBoundary.parse(text, LIMIT)
        require(value.getInt("version") == 1)
        return when (value.getString("kind")) {
            "audio" -> readAudio(value)
            "camera" -> readCamera(value)
            "display" -> readDisplay(value)
            "metadata" -> readMetadata(value)
            "wake" -> WakeTaskJson.decode(value)
            else -> error("Unknown media worker request")
        }
    }
    private fun audio(task: MediaTask.Audio): JSONObject {
        val e = task.endpoint; val p = task.preferences
        return JSONObject().put("kind", "audio").put("token", e.token).put("port", e.port).put("direction", e.direction)
            .put("processing", e.processing).put("buffer", e.bufferMs).put("background", e.background)
            .put("override", p.processing).put("consent", p.background).put("gain", p.gain).put("builtIn", p.builtIn).put("lowLatency", p.speakerLowLatency)
    }
    private fun readAudio(v: JSONObject): MediaTask.Audio {
        val e = AudioEndpoint(v.getString("token"), v.getInt("port"), v.getInt("direction"), v.getInt("processing"), v.getInt("buffer"), v.getBoolean("background"))
        val p = AudioPreferences(v.getInt("override"), v.getBoolean("consent"), v.getInt("gain"), v.getBoolean("builtIn"), v.getBoolean("lowLatency"))
        require(e.valid() && p.valid(e.direction))
        require(p.effective(e) == e) { "Unconfirmed audio settings" }
        return MediaTask.Audio(e, p)
    }
    private fun camera(task: MediaTask.Camera): JSONObject {
        val e = task.endpoint
        return JSONObject().put("kind", "camera").put("token", e.token).put("port", e.port)
            .put("width", e.width).put("height", e.height).put("fps", e.fps).put("bitrate", e.bitrate)
            .put("background", e.background).put("freshness", e.freshnessMs).put("adaptive", e.adaptiveBitrate)
            .put("minimum", e.minBitrate).put("requestedLens", e.requestedLens?.wire).put("lens", task.lens.wire).put("rotation", task.rotation)
    }
    private fun readCamera(v: JSONObject): MediaTask.Camera {
        val e = CameraEndpoint(v.getString("token"), v.getInt("port"), v.getInt("width"), v.getInt("height"), v.getInt("fps"), v.getInt("bitrate"),
            requestedLens = if (v.isNull("requestedLens")) null else CameraLens.entries.single { it.wire == v.getInt("requestedLens") },
            background = v.getBoolean("background"), freshnessMs = v.getInt("freshness"), adaptiveBitrate = v.getBoolean("adaptive"), minBitrate = v.getInt("minimum"))
        val lens = CameraLens.entries.single { it.wire == v.getInt("lens") }
        val rotation = v.getInt("rotation")
        require(e.valid() && rotation in 0..3)
        return MediaTask.Camera(e, lens, rotation)
    }
    private fun display(task: MediaTask.Display): JSONObject {
        val f = task.format
        require(f.codecPrivate == null) { "Codec bytes belong to the video transport" }
        return JSONObject().put("kind", "display").put("token", task.routes.token).put("routeRevision", task.routes.revision)
            .put("videoPort", task.routes.videoPort).put("controlPort", task.routes.controlPort).put("mime", f.mimeType)
            .put("width", f.width).put("height", f.height).put("fps", f.fps).put("selection", selection(f.selection)).put("liveness", task.liveness)
    }
    private fun readDisplay(v: JSONObject): MediaTask.Display {
        val token = v.getString("token")
        require(token.matches(Regex("[0-9a-f]{64}")))
        val format = DecoderFormat(v.getString("mime"), v.getInt("width"), v.getInt("height"), v.getInt("fps"), selection = readSelection(v.optJSONObject("selection")))
        require(format.mimeType in VideoCodec.types.values)
        require(format.width in 1..16384 && format.height in 1..16384 && format.fps in 10..90)
        val routes = DisplayRoutes(routeNumber(v, "routeRevision", Long.MAX_VALUE), token,
            routeNumber(v, "videoPort", 65535).toInt(), routeNumber(v, "controlPort", 65535).toInt())
        val liveness = v.opt("liveness")
        require(liveness == null || liveness is Boolean)
        return MediaTask.Display(routes, format, liveness == true)
    }
    private fun routeNumber(value: JSONObject, name: String, maximum: Long): Long {
        val number = value.get(name)
        require(number is Int || number is Long)
        return (number as Number).toLong().also { require(it in 1..maximum) }
    }
    private fun selection(s: DecoderSelection?): JSONObject? = s?.let {
        JSONObject().put("name", it.name).put("codec", it.codec).put("profile", it.profile).put("level", it.level)
            .put("depth", it.depth).put("low", it.lowLatency).put("rate", it.operatingRate)
    }
    private fun readSelection(v: JSONObject?): DecoderSelection? = v?.let {
        DecoderSelection(it.getString("name"), it.getString("codec"), it.getString("profile"), it.getInt("level"), it.getInt("depth"),
            it.getBoolean("low"), if (it.isNull("rate")) null else it.getInt("rate"))
    }
    private fun readMetadata(v: JSONObject): MediaTask.Metadata {
        val request = MediaTask.Metadata(v.getInt("width"), v.getInt("height"), v.getInt("fps"), v.getInt("shell"))
        require(request.shellVersion in 0..2)
        if (request.shellVersion == 0) require(request.width in 1..16384 && request.height in 1..16384 && request.fps in 1..240)
        return request
    }
}
