package com.blent

import android.content.Context
import android.content.Intent

/** Framework carrier only; original-owner admission remains a separate adapter. */
internal fun StreamingPower.intent(context: Context) = Intent(context, StreamingService::class.java)
    .putExtra("battery_saver", batterySaver).putExtra("stream_connected", active)
    .putExtra("stream_transport", transport.wire).putExtra("display_wake", display.wire)

internal fun StreamingPower.Companion.read(intent: Intent) = StreamingPower(
    intent.getBooleanExtra("battery_saver", false), intent.getBooleanExtra("stream_connected", false),
    StreamTransport.from(intent.getStringExtra("stream_transport")),
    if (intent.hasExtra("display_wake")) DisplayWakeState.from(intent.getStringExtra("display_wake"))
        else DisplayWakeState.WAITING,
)
