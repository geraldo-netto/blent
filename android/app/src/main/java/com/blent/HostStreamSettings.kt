package com.blent

/** One paired encoder-settings observation, independent of decoder/UI state. */
internal data class HostStreamSettings(val bitrateKbps: Int, val fps: Int) {
    init {
        require(bitrateKbps in 1000..60000)
        require(fps in 10..90)
    }
}

/** Delivery identity rejects observations from an original retired connection. */
internal data class HostSettingsObservation(val values: HostStreamSettings, val source: ControlNotice)
