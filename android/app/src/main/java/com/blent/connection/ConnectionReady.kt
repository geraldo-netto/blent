package com.blent.connection

/** Public approval scheduling only; this marker grants no peer or media authority.
 * Mutual TLS and the current approved ticket remain mandatory afterward. */
internal object ConnectionReady {
    const val BYTES = 8
    fun encode(host: Boolean): ByteArray = byteArrayOf(66, 76, 80, 82, 1, if (host) 1 else 0, 0, 0)
    fun decode(bytes: ByteArray, expectHost: Boolean) {
        require(bytes.contentEquals(encode(expectHost))) { "Invalid approval-ready marker" }
    }
}
