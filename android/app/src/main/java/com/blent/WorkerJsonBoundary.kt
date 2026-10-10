package com.blent

import org.json.JSONObject

/** Bound recursive JSON parsing as well as bytes; worker corruption must not
 * turn a small, deeply nested reply into a main-process stack overflow. */
internal object WorkerJsonBoundary {
    fun parse(text: String, limit: Int): JSONObject {
        require(text.length <= limit)
        val scan = Scanner()
        text.forEach(scan::accept)
        scan.finish()
        return JSONObject(text)
    }
    private class Scanner {
        private var depth = 0
        private var quoted = false
        private var escaped = false
        fun accept(char: Char) {
            if (quoted) string(char)
            else when (char) {
                '"' -> quoted = true
                '{', '[' -> { depth++; require(depth <= 16) { "Worker JSON nesting exceeded" } }
                '}', ']' -> { depth--; require(depth >= 0) }
            }
        }
        private fun string(char: Char) {
            when {
                escaped -> escaped = false
                char == '\\' -> escaped = true
                char == '"' -> quoted = false
            }
        }
        fun finish() { require(depth == 0 && !quoted) }
    }
}
