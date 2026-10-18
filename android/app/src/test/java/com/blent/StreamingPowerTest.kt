package com.blent

import android.content.Intent
import android.net.wifi.WifiManager
import kotlinx.coroutines.runBlocking
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.Shadows.shadowOf
import org.robolectric.annotation.Config
import org.robolectric.annotation.Implementation
import org.robolectric.annotation.Implements
import org.robolectric.shadow.api.Shadow
import org.robolectric.shadows.ShadowPowerManager

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [27, 34], shadows = [PowerTestWifiManager::class, PowerTestWifiLock::class])
class StreamingPowerTest {
    private fun cpuHeld() = ShadowPowerManager.getLatestWakeLock()?.isHeld == true
    private fun wifiHeld() = PowerTestWifiManager.latest?.isHeld == true

    @Test fun t497_foregroundServiceDoesNotExposeAnIpcBinder() = runBlocking {
        val rig = NativePowerServiceFixture()
        try { assertNull(rig.service.onBind(Intent())) }
        finally { rig.close() }
    }
    @Test fun t388_battery_profile_releases_usb_and_waiting_locks() = runBlocking {
        val rig = NativePowerServiceFixture()
        try {
            rig.update(StreamingPower(true, true, StreamTransport.USB))
            assertFalse("T388: USB battery profile must not hold the Wi-Fi radio", wifiHeld())
            assertFalse("T388: visible Activity already keeps the screen/device awake", cpuHeld())
        } finally { rig.close() }
    }
    @Test fun t388_network_disconnect_hysteresis_cancels_on_recovery_and_destroy() = runBlocking {
        val rig = NativePowerServiceFixture()
        val looper = shadowOf(android.os.Looper.getMainLooper())
        try {
            rig.update(StreamingPower(true, true, StreamTransport.NETWORK)); rig.ready(1)
            assertTrue(wifiHeld()); assertFalse(cpuHeld())
            rig.update(StreamingPower(true, false))
            looper.idleFor(java.time.Duration.ofSeconds(4))
            assertTrue(wifiHeld())
            rig.update(StreamingPower(true, false))
            looper.idleFor(java.time.Duration.ofSeconds(1))
            rig.until { !wifiHeld() }; assertFalse(wifiHeld())
            rig.update(StreamingPower(true, true)); rig.ready(2)
            assertTrue(wifiHeld())
            rig.update(StreamingPower(true, false))
            looper.idleFor(java.time.Duration.ofSeconds(4))
            rig.update(StreamingPower(true, true, StreamTransport.NETWORK))
            looper.idleFor(java.time.Duration.ofSeconds(2))
            assertTrue(wifiHeld())
            rig.update(StreamingPower(true, true, StreamTransport.USB))
            rig.until { !wifiHeld() }; assertFalse(wifiHeld())
            rig.update(StreamingPower(true, true, StreamTransport.NETWORK)); rig.ready(3)
            rig.update(StreamingPower(true, false))
        } finally { rig.close() }
        looper.idleFor(java.time.Duration.ofSeconds(6))
        assertFalse(wifiHeld()); assertFalse(cpuHeld())
    }
    @Test fun t1060PausedDisplayReleasesBothNativeLocksWithoutIdleGrace() = runBlocking {
        val rig = NativePowerServiceFixture()
        try {
            rig.update(StreamingPower(false, true, StreamTransport.NETWORK)); rig.ready(1)
            assertTrue(cpuHeld()); assertTrue(wifiHeld())
            rig.update(StreamingPower(false, true, StreamTransport.NETWORK, DisplayWakeState.PAUSED))
            rig.until { !cpuHeld() && !wifiHeld() }
            assertFalse("T1060 paused display retained native CPU wake", cpuHeld())
            assertFalse("T1060 paused display retained native Wi-Fi wake through idle grace", wifiHeld())
        } finally { rig.close() }
    }
    @Test fun t388_foreground_rejection_releases_previously_acquired_locks() = runBlocking {
        val rig = NativePowerServiceFixture()
        try {
            rig.update(StreamingPower(false, true, StreamTransport.NETWORK)); rig.ready(1)
            shadowOf(rig.service).setThrowInStartForeground(SecurityException("T388 rejected"))
            rig.update(StreamingPower(false, true, StreamTransport.NETWORK))
            rig.until { !cpuHeld() && !wifiHeld() }
            assertFalse(wifiHeld()); assertFalse(cpuHeld())
            assertTrue(shadowOf(rig.service).isStoppedBySelf)
        } finally { rig.close() }
    }
    @Test fun t388_foreground_promotion_does_not_wait_for_start_command() = runBlocking {
        val rig = NativePowerServiceFixture()
        try { assertNotNull(shadowOf(rig.service).lastForegroundNotification) }
        finally { rig.close() }
    }
    @Test fun t388_normal_mode_retains_existing_locks_and_destroy_releases_them() = runBlocking {
        val rig = NativePowerServiceFixture()
        try {
            rig.update(StreamingPower()); rig.ready(1)
            rig.update(StreamingPower())
            assertTrue(cpuHeld()); assertTrue(wifiHeld())
        } finally { rig.close() }
        assertFalse(cpuHeld()); assertFalse(wifiHeld())
    }
}

/** T388: isolate lock ownership from Robolectric 4.17's newer BlockingOption API. */
@Implements(WifiManager::class)
class PowerTestWifiManager {
    @Implementation
    @Suppress("DEPRECATION")
    fun createWifiLock(mode: Int, tag: String): WifiManager.WifiLock {
        check(mode == WifiManager.WIFI_MODE_FULL_LOW_LATENCY || mode == WifiManager.WIFI_MODE_FULL_HIGH_PERF)
        check(tag == "Blent::StreamingWifiLock")
        return Shadow.newInstanceOf(WifiManager.WifiLock::class.java).also { latest = it }
    }
    companion object { @Volatile var latest: WifiManager.WifiLock? = null }
}

@Implements(WifiManager.WifiLock::class)
class PowerTestWifiLock {
    @Volatile private var held = false
    @Implementation fun setReferenceCounted(enabled: Boolean) { check(!enabled) }
    @Implementation fun acquire() { held = true }
    @Implementation fun release() { check(held); held = false }
    @Implementation fun isHeld(): Boolean = held
}
