package com.blent

import android.net.wifi.WifiManager
import android.os.PowerManager
import java.util.concurrent.CopyOnWriteArrayList
import org.junit.After
import org.junit.Assert.*
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config
import org.robolectric.annotation.Implementation
import org.robolectric.annotation.Implements
import org.robolectric.shadow.api.Shadow

/** Native-model bookkeeping is independent of the application cleanup owner. */
internal object NativePowerFaults {
    @Volatile var acquireCpu = false
    @Volatile var releaseCpu = false
    val cpu = CopyOnWriteArrayList<FaultCpuWakeLock>()
    val wifi = CopyOnWriteArrayList<FaultWifiWakeLock>()
    fun cleanup() {
        acquireCpu = false; releaseCpu = false
        cpu.forEach { it.live = false }; wifi.forEach { it.live = false }
        cpu.clear(); wifi.clear()
    }
}

@Implements(PowerManager::class)
class FaultPowerManager {
    @Implementation fun newWakeLock(level: Int, tag: String): PowerManager.WakeLock {
        check(level == PowerManager.PARTIAL_WAKE_LOCK && tag == "Blent::StreamingWakeLock")
        return Shadow.newInstanceOf(PowerManager.WakeLock::class.java)
    }
}
@Implements(PowerManager.WakeLock::class)
class FaultCpuWakeLock {
    @Volatile var live = false
    @Volatile var grants = 0
    init { NativePowerFaults.cpu.add(this) }
    @Implementation fun setReferenceCounted(enabled: Boolean) { check(!enabled) }
    @Implementation fun acquire(timeout: Long) {
        check(timeout == CPU_WAKE_MILLIS)
        grants++
        live = true
        check(!NativePowerFaults.acquireCpu) { "T1060 CPU granted before API failure" }
    }
    @Implementation fun release() {
        check(!NativePowerFaults.releaseCpu) { "T1060 original CPU release uncertain" }
        live = false
    }
    @Implementation fun isHeld() = live
}
@Implements(WifiManager::class)
class FaultWifiManager {
    @Implementation fun createWifiLock(mode: Int, tag: String): WifiManager.WifiLock {
        check(mode == WifiManager.WIFI_MODE_FULL_LOW_LATENCY || mode == WifiManager.WIFI_MODE_FULL_HIGH_PERF)
        check(tag == "Blent::StreamingWifiLock")
        return Shadow.newInstanceOf(WifiManager.WifiLock::class.java)
    }
}
@Implements(WifiManager.WifiLock::class)
class FaultWifiWakeLock {
    @Volatile var live = false
    init { NativePowerFaults.wifi.add(this) }
    @Implementation fun setReferenceCounted(enabled: Boolean) { check(!enabled) }
    @Implementation fun acquire() { live = true }
    @Implementation fun release() { live = false }
    @Implementation fun isHeld() = live
}

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [27, 34], shadows = [FaultPowerManager::class, FaultCpuWakeLock::class, FaultWifiManager::class, FaultWifiWakeLock::class])
class NativePowerFaultTest {
    @Before fun reset() = NativePowerFaults.cleanup()
    @After fun cleanup() = NativePowerFaults.cleanup()

    @Test fun t1060GrantThenAcquireFailureRetainsOriginalCpuCleanupOwnership() = kotlinx.coroutines.runBlocking {
        val rig = NativePowerServiceFixture()
        NativePowerFaults.acquireCpu = true
        try {
            rig.update(StreamingPower())
            rig.ended(1)
            assertEquals("T1060 fixture must grant before the injected failure", 1, NativePowerFaults.cpu.sumOf { it.grants })
            rig.destroy()
            rig.until { NativePowerFaults.cpu.none { it.live } }
            assertFalse("T1060 native CPU grant lost its cleanup owner after acquire threw", NativePowerFaults.cpu.any { it.live })
        } finally { NativePowerFaults.acquireCpu = false; rig.close() }
    }
    @Test fun t1060CpuReleaseFailureCannotSkipIndependentWifiCleanup() = kotlinx.coroutines.runBlocking {
        val rig = NativePowerServiceFixture()
        try {
            rig.update(StreamingPower()); rig.ready(1)
            assertTrue(NativePowerFaults.cpu.any { it.live }); assertTrue(NativePowerFaults.wifi.any { it.live })
            NativePowerFaults.releaseCpu = true
            rig.destroy()
            rig.until { NativePowerFaults.wifi.none { it.live } }
            assertFalse("T1060 CPU release failure skipped Wi-Fi cleanup", NativePowerFaults.wifi.any { it.live })
            assertTrue("T1060 missing original CPU retirement was erased", NativePowerFaults.cpu.any { it.live })
        } finally { NativePowerFaults.releaseCpu = false; rig.close() }
    }
}
