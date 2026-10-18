package com.blent

import org.json.JSONObject

/** Fixed private-worker values. Never carries a native handle or authority. */
internal object WakeTaskJson {
    fun encode(task: MediaTask.Wake): JSONObject = JSONObject().put("kind", "wake").put("cpu", task.cpu).put("wifi", task.wifi)
    fun decode(value: JSONObject): MediaTask.Wake {
        require(value.keys().asSequence().toSet() == setOf("version", "kind", "cpu", "wifi"))
        require(value.get("version") == 1 && value.get("version") is Int && value.get("kind") == "wake")
        return MediaTask.Wake(boolean(value, "cpu"), boolean(value, "wifi"))
    }
    fun encodeReady(event: WorkerEvent.WakeReady): JSONObject = JSONObject().put("type", "wake_ready").put("cpu", event.cpu).put("wifi", event.wifi)
    fun decodeReady(value: JSONObject): WorkerEvent.WakeReady {
        require(value.keys().asSequence().toSet() == setOf("type", "cpu", "wifi"))
        require(value.get("type") == "wake_ready")
        return WorkerEvent.WakeReady(boolean(value, "cpu"), boolean(value, "wifi"))
    }
    private fun boolean(value: JSONObject, name: String): Boolean {
        val result = value.get(name)
        require(result is Boolean)
        return result
    }
}
