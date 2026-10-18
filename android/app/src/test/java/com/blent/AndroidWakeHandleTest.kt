package com.blent

import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.RuntimeEnvironment
import org.robolectric.annotation.Config
import org.robolectric.util.ReflectionHelpers

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [27, 34], shadows = [PowerTestWifiManager::class, PowerTestWifiLock::class])
class AndroidWakeHandleTest {
    private class Lock : PlatformWakeLock {
        override var held = false
        var acquireFailure: Exception? = null
        var releaseFailure: Exception? = null
        var grant = true
        var releases = 0
        override fun acquire() { held = grant; acquireFailure?.let { throw it } }
        override fun release() { releases++; releaseFailure?.let { throw it }; held = false }
    }
    @Test fun t1060UncertainUnheldAcquisitionCannotManufactureNativeRetirement() {
        val lock = Lock().apply { grant = false; acquireFailure = IllegalStateException("T1060 no original API response") }
        val handle = AndroidWakeHandle(lock)
        handle.retire()
        assertTrue(runCatching { handle.acquire() }.isFailure)
        assertTrue(runCatching { handle.retire() }.isFailure)
        assertEquals(0, lock.releases)
        lock.held = true // Supporting original API state permits an actual release call.
        handle.retire(); assertEquals(1, lock.releases)
        assertFalse(lock.held)
    }
    @Test fun t1060OriginalApiReleaseFailureRetainsAdmissionAndRetryRequiresEvidence() {
        val lock = Lock()
        val handle = AndroidWakeHandle(lock)
        handle.acquire()
        assertTrue(runCatching { handle.acquire() }.isFailure)
        lock.releaseFailure = IllegalStateException("T1060 original release uncertain")
        assertTrue(runCatching { handle.retire() }.isFailure)
        lock.held = false
        assertTrue(runCatching { handle.retire() }.isFailure)
        assertEquals(1, lock.releases)
        lock.held = true; lock.releaseFailure = null
        handle.retire(); handle.retire()
        assertEquals(2, lock.releases)
        assertFalse(lock.held)
    }
    @Test fun t1060NativeFactoriesUseOriginalCpuAndWifiApiHandles() {
        val app = RuntimeEnvironment.getApplication()
        for (handle in listOf(AndroidWakeHandles.cpu(app), AndroidWakeHandles.wifi(app))) {
            val native = ReflectionHelpers.getField<PlatformWakeLock>(handle, "lock")
            assertFalse(native.held)
            handle.acquire()
            try { assertTrue(native.held) }
            finally { handle.retire() }
            assertFalse(native.held)
        }
        assertEquals(14400000L, CPU_WAKE_MILLIS)
    }
}
