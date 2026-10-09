package com.blent

internal data class AudioPreferences(val processing: Int = 0, val background: Boolean = false,
    val gain: Int = 100, val builtIn: Boolean = true, val speakerLowLatency: Boolean = false) {
    fun valid(direction: Int = 1): Boolean = direction in 1..2 && processing in 0..2 && gain in 0..(if (direction == 1) 200 else 100)
    fun effective(endpoint: AudioEndpoint): AudioEndpoint {
        require(valid(endpoint.direction))
        require(!endpoint.background || background) { "Allow background ${if (endpoint.direction == 1) "microphone" else "speaker"} sharing on the tablet first." }
        return endpoint.copy(processing = if (processing == 0) endpoint.processing else processing)
    }
}
