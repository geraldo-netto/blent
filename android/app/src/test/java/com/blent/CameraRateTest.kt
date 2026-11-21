package com.blent

import org.junit.Assert.*
import org.junit.Test

class CameraRateTest {
    private fun rate(enabled: Boolean = true) = CameraRate(3000, 1000, enabled, 150_000)

    @Test fun t618_pressureUsesHysteresisAndSlowBoundedRecovery() {
        val rate = rate()
        assertNull(rate.observe(120_000, 0))
        assertNull(rate.observe(120_000, 100_000))
        assertNull(rate.observe(120_000, 200_000))
        assertEquals(2250, rate.observe(120_000, 1_000_000))
        assertNull(rate.observe(0, 2_000_000))
        assertNull(rate.observe(0, 6_999_999))
        assertEquals(2475, rate.observe(0, 7_000_000))
        assertNull(rate.observe(0, 8_000_000))
        assertNull(rate.observe(80_000, 12_000_000)) // neutral breaks healthy run
        assertNull(rate.observe(0, 13_000_000))
        assertEquals(2722, rate.observe(0, 18_000_000))
        for (i in 1L..20) { rate.observe(0, 20_000_000 + i * 6_000_000) }
        assertEquals(3000, rate.current)
    }

    @Test fun t618_timeoutFloorAndFixedModeStayWithinUserLimits() {
        val rate = rate()
        assertEquals(2250, rate.congested(0))
        assertEquals(1687, rate.congested(1))
        assertEquals(1265, rate.congested(2))
        assertEquals(1000, rate.congested(3))
        assertNull(rate.congested(4))
        val fixed = rate(false)
        repeat(100) { assertNull(fixed.observe(999_999, it.toLong())); assertNull(fixed.congested(it.toLong())) }
        assertEquals(3000, fixed.current)
        val low = CameraRate(256, 1000, true, 50_000)
        assertNull(low.congested(0)); assertEquals(256, low.current)
        assertTrue(runCatching { rate.observe(-1, 0) }.isFailure)
        assertTrue(runCatching { rate.observe(0, -1) }.isFailure)
        assertTrue(runCatching { rate.congested(-1) }.isFailure)
        for (invalid in listOf(Int.MIN_VALUE, 0, 255, 20001, Int.MAX_VALUE)) {
            assertTrue(runCatching { CameraRate(invalid, 1000, true, 150_000) }.isFailure)
            assertTrue(runCatching { CameraRate(3000, invalid, true, 150_000) }.isFailure)
        }
        assertTrue(runCatching { CameraRate(3000, 1000, true, 0) }.isFailure)
    }
    // T1477 native adapter boundary: a failed setParameters must not commit policy.
    private fun configureNative(rate: CameraRate, native: (Int) -> Unit) {
        rate.configure(5000, 4500, false, 1, native)
    }

    @Test fun t1477_failedNativeConfigurationPreservesPolicyAndRetryReassertsTarget() {
        val rate = rate()
        assertThrows(IllegalStateException::class.java) {
            configureNative(rate) { throw IllegalStateException("native failure after possible effect") }
        }
        assertEquals("T1477 unacknowledged target is not current policy", 3000, rate.current)
        assertEquals("T1477 original adaptive mode and floor survive", 2250, rate.congested(2))
        var applied = 0
        configureNative(rate) { applied = it }
        assertEquals(5000, applied)
        assertEquals(5000, rate.current)
    }

    @Test fun t1477_revertingAfterUncertainNativeEffectReassertsUnchangedTarget() {
        val rate = rate()
        var actual = 3000
        assertThrows(IllegalStateException::class.java) {
            configureNative(rate) { actual = it; throw IllegalStateException("after effect") }
        }
        assertEquals(5000, actual)
        assertNull(rate.configure(3000, 1000, true, 2) { actual = it })
        assertEquals(3000, actual)
        assertEquals(3000, rate.current)
    }
}
