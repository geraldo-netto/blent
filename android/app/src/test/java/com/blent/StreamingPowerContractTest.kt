package com.blent

import org.junit.Assert.*
import org.junit.Test

class StreamingPowerContractTest {
    private fun expectedWifi(saver: Boolean, active: Boolean, transport: StreamTransport): Boolean {
        if (!saver) return true
        if (!active) return false
        return when (transport) {
            StreamTransport.USB -> false
            StreamTransport.NETWORK, StreamTransport.UNKNOWN -> true
        }
    }
    private fun assertPolicy(state: DisplayWakeState, awake: Boolean) {
        for (saver in listOf(false, true)) {
            for (active in listOf(false, true)) {
                for (transport in StreamTransport.entries) {
                    val policy = StreamingPower(saver, active, transport, state)
                    val wifi = awake && expectedWifi(saver, active, transport)
                    assertEquals("T1059 $policy", awake, policy.keepScreenOn)
                    assertEquals("T1059 $policy", awake && !saver, policy.needsCpu)
                    assertEquals("T1059 $policy", wifi, policy.needsWifi)
                    assertEquals("T1059 $policy", expectedDemand(awake, wifi, transport), policy.wifiDemand)
                }
            }
        }
    }
    private fun expectedDemand(awake: Boolean, wifi: Boolean, transport: StreamTransport): WifiWakeDemand {
        if (!awake) return WifiWakeDemand.RELEASE
        if (wifi) return WifiWakeDemand.HOLD
        if (transport == StreamTransport.USB) return WifiWakeDemand.RELEASE
        return WifiWakeDemand.IDLE_GRACE
    }
    @Test fun t1059EveryDisplayTransportAndEnergyStateHasIndependentWakePolicy() {
        for (state in listOf(DisplayWakeState.WAITING, DisplayWakeState.UNSUPPORTED, DisplayWakeState.ACTIVE)) assertPolicy(state, true)
        for (state in listOf(DisplayWakeState.PAUSED, DisplayWakeState.UNAVAILABLE, DisplayWakeState.BACKGROUND)) assertPolicy(state, false)
    }
    @Test fun t1059UnknownDisplayValuesDenyWakeWithoutInventingTransportSupport() {
        for (state in DisplayWakeState.entries) assertEquals(state, DisplayWakeState.from(state.wire))
        for (invalid in listOf(null, "", "ACTIVE", "paused ", "future", "x".repeat(4096))) assertEquals(DisplayWakeState.UNAVAILABLE, DisplayWakeState.from(invalid))
        for (invalid in listOf(null, "", "USB", "future", "x".repeat(4096))) assertEquals(StreamTransport.UNKNOWN, StreamTransport.from(invalid))
    }
}
