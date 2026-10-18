package com.blent

import com.blent.activity.DisplayActivityState
import com.blent.activity.DisplayActivityUpdate

/** One synchronous bounded policy input: three booleans and an optional immutable
 * original observation. Caller owns its containing snapshot; no native allocation. */
internal data class DisplayWakeSnapshot(
    val foreground: Boolean,
    val authenticated: Boolean,
    val metadataCurrent: Boolean,
    val observation: DisplayActivityUpdate?,
)

internal fun DisplayWakeSnapshot.wakeState(): DisplayWakeState {
    if (!foreground) return DisplayWakeState.BACKGROUND
    return when (val state = observation?.state) {
        null -> DisplayWakeState.WAITING
        DisplayActivityState.Unavailable -> if (observation?.supportKnown == true) DisplayWakeState.UNAVAILABLE else DisplayWakeState.WAITING
        DisplayActivityState.Unsupported -> DisplayWakeState.UNSUPPORTED
        is DisplayActivityState.Observed -> observedWake(state.signal.active)
    }
}
private fun DisplayWakeSnapshot.observedWake(active: Boolean): DisplayWakeState {
    if (!authenticated || !metadataCurrent) return DisplayWakeState.UNAVAILABLE
    return if (active) DisplayWakeState.ACTIVE else DisplayWakeState.PAUSED
}
