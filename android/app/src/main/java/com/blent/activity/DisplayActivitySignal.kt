package com.blent.activity

import org.json.JSONObject

/** One original authenticated control connection owns these counters. This
 * observation supplies no media permission or proof of native retirement. */
internal data class DisplayActivitySignal(val revision: Long, val stopped: Long, val active: Boolean) {
    init { require(revision > 0 && stopped >= 0 && stopped <= revision) }
    companion object {
        const val VERSION = 1
        fun read(value: JSONObject): DisplayActivitySignal {
            require(value.length() == 4 && integer(value, "version") == VERSION.toLong())
            val active = value.get("active")
            require(active is Boolean)
            return DisplayActivitySignal(integer(value, "revision"), integer(value, "stopped"), active)
        }
        private fun integer(value: JSONObject, field: String): Long = when (val raw = value.get(field)) {
            is Int -> raw.toLong()
            is Long -> raw
            else -> throw IllegalArgumentException("$field must be an integer")
        }
    }
}
