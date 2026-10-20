package com.blent

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent

internal object CameraEndpointIntent {
    fun read(intent: Intent): CameraEndpoint? {
        val lens = intent.getIntExtra("lens", -1)
        if (lens !in 0..1) return null
        val candidate = CameraEndpoint(intent.getStringExtra("token") ?: "", intent.getIntExtra("port", 0),
            intent.getIntExtra("width", 0), intent.getIntExtra("height", 0),
            intent.getIntExtra("fps", 0), intent.getIntExtra("bitrate", 0),
            CameraLens.values().firstOrNull { it.wire == lens }, intent.getBooleanExtra("background", false), intent.getIntExtra("freshness_ms", 150),
            intent.getBooleanExtra("adaptive_bitrate", true), intent.getIntExtra("min_bitrate", 1000))
        return candidate.takeIf { it.valid() }
    }
}

internal object CameraStopIntent {
    const val ACTION = "com.blent.CAMERA_STOP"

    fun read(intent: Intent): CameraStop? {
        if (intent.action != ACTION) return null
        val credential = intent.getStringExtra("expected_token") ?: return null
        return CameraStop.create(credential, intent.getIntExtra("expected_port", 0))
    }
}

class CameraReceiver : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
        when (intent.action) {
            null -> {
                val endpoint = CameraEndpointIntent.read(intent) ?: return
                CameraInvitations.endpoint.value = endpoint
            }
            CameraStopIntent.ACTION -> {
                val request = CameraStopIntent.read(intent) ?: return
                CameraInvitations.stop(request)
            }
            else -> return
        }
        // Processed cutoff only: the camera owner retains actual native cleanup.
        resultCode = 1
    }
}
