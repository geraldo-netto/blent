package com.blent

/** Explicit user intent only; absent fields belong to the host. */
internal data class StreamOverrides(val bitrateKbps: Int? = null, val fps: Int? = null) {
    init {
        require(bitrateKbps == null || bitrateKbps in 1000..60000)
        require(fps == null || fps in 10..90)
    }
    val empty get() = bitrateKbps == null && fps == null
    fun merge(change: StreamOverrides) = StreamOverrides(change.bitrateKbps ?: bitrateKbps, change.fps ?: fps)
    fun differentFrom(bitrate: Int, rate: Int) = StreamOverrides(
        bitrateKbps?.takeIf { it != bitrate }, fps?.takeIf { it != rate },
    )
    fun confirmed(values: HostStreamSettings) = StreamOverrides(
        bitrateKbps?.let { values.bitrateKbps }, fps?.let { values.fps },
    )
}
