package com.blent

internal enum class CameraLens(val wire: Int, val label: String) { FRONT(0, "Front"), REAR(1, "Rear") }

internal data class CameraEndpoint(val token: String, val port: Int, val width: Int, val height: Int, val fps: Int, val bitrate: Int,
    val requestedLens: CameraLens? = null, val background: Boolean = false, val freshnessMs: Int = 150,
    val adaptiveBitrate: Boolean = true, val minBitrate: Int = 1000) {
    fun valid(): Boolean = token.matches(Regex("[0-9a-f]{64}")) && port in 1..65535 && validProfile() && validFreshness()

    private fun validFreshness() = freshnessMs in 50..2000 && minBitrate in 256..20000

    private fun validProfile(): Boolean = width in 160..1920 && height in 120..1080 &&
        width % 2 == 0 && height % 2 == 0 && fps in 5..30 && bitrate in 256..20000

}
