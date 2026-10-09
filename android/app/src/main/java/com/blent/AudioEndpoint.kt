package com.blent

internal data class AudioEndpoint(val token: String, val port: Int, val direction: Int,
    val processing: Int, val bufferMs: Int, val background: Boolean) {
    fun valid(): Boolean = token.matches(Regex("[0-9a-f]{64}")) && port in 1..65535 &&
        direction in 1..2 && processing in 1..2 && bufferMs in 20..200 && bufferMs % 10 == 0
}
