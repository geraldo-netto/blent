package com.blent

import android.content.Intent
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.RuntimeEnvironment
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [27, 34])
class StreamingPowerIntentTest {
    @Test fun t1059FrameworkBoundaryPreservesEveryIndependentPolicyValue() {
        for (display in DisplayWakeState.entries) {
            for (transport in StreamTransport.entries) {
                for (saver in listOf(false, true)) {
                    for (active in listOf(false, true)) {
                        val original = StreamingPower(saver, active, transport, display)
                        val intent = original.intent(RuntimeEnvironment.getApplication())
                        assertEquals(StreamingService::class.java.name, intent.component?.className)
                        assertEquals(original, StreamingPower.read(intent))
                    }
                }
            }
        }
    }
    @Test fun t1059MissingLegacyObservationKeepsExplicitWaitingFallback() {
        assertEquals(StreamingPower(), StreamingPower.read(Intent()))
        val legacy = Intent().putExtra("battery_saver", true).putExtra("stream_connected", true).putExtra("stream_transport", "usb")
        assertEquals(StreamingPower(true, true, StreamTransport.USB), StreamingPower.read(legacy))
    }
    @Test fun t1059MalformedDisplayCarrierCannotAcquireWakeResources() {
        for (invalid in listOf(null, "", "future", "ACTIVE", "x".repeat(4096))) {
            val power = StreamingPower.read(Intent().putExtra("display_wake", invalid))
            assertEquals(DisplayWakeState.UNAVAILABLE, power.display)
            assertFalse("T1059 malformed carrier acquired CPU", power.needsCpu)
            assertFalse("T1059 malformed carrier acquired Wi-Fi", power.needsWifi)
            assertFalse("T1059 malformed carrier kept screen on", power.keepScreenOn)
        }
        val wrongType = StreamingPower.read(Intent().putExtra("display_wake", 1))
        assertEquals(DisplayWakeState.UNAVAILABLE, wrongType.display)
        assertEquals(WifiWakeDemand.RELEASE, wrongType.wifiDemand)
    }
}
