// Review-only platform/Compose/value stubs; production owner classes are copied unchanged.
package com.blent

import android.content.Context
import java.net.Socket
import kotlinx.coroutines.flow.MutableStateFlow

internal data class AudioEndpoint(val port: Int, val direction: Int = 1, val background: Boolean = false) {
    fun valid() = port in 1..65535
}
internal class AudioPreferences {
    companion object { fun load(context: Context, direction: Int) = AudioPreferences() }
    fun effective(endpoint: AudioEndpoint) = endpoint
    fun valid(direction: Int) = direction in 1..2
    fun save(context: Context, direction: Int) {}
}
internal object AudioInvitations { val microphone = MutableStateFlow<AudioEndpoint?>(null) }
internal class AudioCapture(context: Context) {
    suspend fun run(endpoint: AudioEndpoint, prefs: AudioPreferences, socket: (Socket?) -> Unit, status: (String) -> Unit) {
        error("Probe must inject capture")
    }
}
internal enum class CameraLens(val label: String) { FRONT("Front"), REAR("Rear") }
internal data class CameraEndpoint(val requestedLens: CameraLens? = CameraLens.FRONT, val background: Boolean = false)
internal object CameraInvitations { val endpoint = MutableStateFlow<CameraEndpoint?>(null) }
internal class CameraCapture(context: Context) {
    suspend fun run(endpoint: CameraEndpoint, lens: CameraLens, rotation: Int, resources: CameraResources) {
        error("Probe must inject capture")
    }
}
