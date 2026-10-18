package com.blent

/** Host-reported ADB route; loopback and charging state cannot identify it. */
internal enum class StreamTransport(val wire: String) {
    UNKNOWN("unknown"), USB("usb"), NETWORK("network");
    companion object {
        fun from(value: String?) = entries.firstOrNull { it.wire == value } ?: UNKNOWN
    }
}

internal data class ControlConnection(val authenticated: Boolean = false, val transport: StreamTransport = StreamTransport.UNKNOWN)

/** Original display observation, separate from transport/media authentication. */
internal enum class DisplayWakeState(val wire: String) {
    WAITING("waiting"), UNSUPPORTED("unsupported"), ACTIVE("active"),
    PAUSED("paused"), UNAVAILABLE("unavailable"), BACKGROUND("background");

    val allowsWake get() = when (this) {
        WAITING, UNSUPPORTED, ACTIVE -> true
        PAUSED, UNAVAILABLE, BACKGROUND -> false
    }
    companion object {
        fun from(value: String?) = entries.firstOrNull { it.wire == value } ?: UNAVAILABLE
    }
}

internal enum class WifiWakeDemand { HOLD, RELEASE, IDLE_GRACE }

/** Bounded immutable desired policy; never native lock/retirement authority. */
internal data class StreamingPower(
    val batterySaver: Boolean = false,
    val active: Boolean = false,
    val transport: StreamTransport = StreamTransport.UNKNOWN,
    val display: DisplayWakeState = DisplayWakeState.WAITING,
) {
    val keepScreenOn get() = display.allowsWake
    val needsCpu get() = keepScreenOn && !batterySaver
    val needsWifi get() = keepScreenOn && (!batterySaver || (active && transport != StreamTransport.USB))
    val wifiDemand get() = when {
        needsWifi -> WifiWakeDemand.HOLD
        !keepScreenOn || transport == StreamTransport.USB -> WifiWakeDemand.RELEASE
        else -> WifiWakeDemand.IDLE_GRACE
    }
    companion object
}
