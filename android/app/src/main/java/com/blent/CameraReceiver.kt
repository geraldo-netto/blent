package com.blent

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import kotlinx.coroutines.flow.MutableStateFlow

internal object CameraEndpointIntent {
    fun read(intent: Intent): CameraEndpoint? {
        val lens = intent.getIntExtra("lens", -1)
        if (lens !in -1..1) return null
        val candidate = CameraEndpoint(intent.getStringExtra("token") ?: "", intent.getIntExtra("port", 0),
            intent.getIntExtra("width", 0), intent.getIntExtra("height", 0),
            intent.getIntExtra("fps", 0), intent.getIntExtra("bitrate", 0),
            CameraLens.values().firstOrNull { it.wire == lens }, intent.getBooleanExtra("background", false), intent.getIntExtra("freshness_ms", 150),
            intent.getBooleanExtra("adaptive_bitrate", true), intent.getIntExtra("min_bitrate", 1000))
        return candidate.takeIf { it.valid() }
    }
}

/** Memory-only host commands. Legacy invitations without a lens never start capture. */
internal object CameraInvitations { val endpoint = MutableStateFlow<CameraEndpoint?>(null) }

class CameraReceiver : BroadcastReceiver() {
override fun onReceive(context: Context, intent: Intent) {
    val endpoint = CameraEndpointIntent.read(intent) ?: return
    CameraInvitations.endpoint.value = endpoint
    resultCode = 1
}
}
