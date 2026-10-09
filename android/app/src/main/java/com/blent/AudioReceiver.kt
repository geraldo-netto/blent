package com.blent

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import kotlinx.coroutines.flow.MutableStateFlow

internal object AudioEndpointIntent {
    fun read(intent: Intent): AudioEndpoint? = AudioEndpoint(intent.getStringExtra("token") ?: "",
        intent.getIntExtra("port", 0), intent.getIntExtra("direction", 0),
        intent.getIntExtra("processing", 0), intent.getIntExtra("buffer_ms", 0),
        intent.getBooleanExtra("background", false)).takeIf { it.valid() }
}
internal object AudioInvitations {
val microphone = MutableStateFlow<AudioEndpoint?>(null)
val speakers = MutableStateFlow<AudioEndpoint?>(null)
}
class AudioReceiver : BroadcastReceiver() {
override fun onReceive(context: Context, intent: Intent) {
    val endpoint = AudioEndpointIntent.read(intent) ?: return
    if (endpoint.direction == 1) AudioInvitations.microphone.value = endpoint
    else AudioInvitations.speakers.value = endpoint
    resultCode = 1
}
}
