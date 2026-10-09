package com.blent

import org.junit.Assert.*
import org.junit.Test

class AudioDriftCacheTest {
    private fun completed(sourceFrames: Long = 96048, destinationFrames: Long = 96000): AudioClockDrift {
        val drift = AudioClockDrift()
        drift.source.observe(AudioClockSample(1, 0, 1), 0)
        drift.destination.observe(AudioClockSample(2, 0, 900_000_000_001), 0)
        drift.source.observe(AudioClockSample(1, sourceFrames, 2_000_000_001), 2000)
        drift.destination.observe(AudioClockSample(2, destinationFrames, 902_000_000_001), 2000)
        return drift
    }

    @Test fun t796_cachedRatesNeverBypassFreshnessOrUnavailableWindows() {
        val drift = completed()
        repeat(50) { assertEquals(500, drift.correction(2000 + it.toLong())) }
        assertEquals(500, drift.correction(5000))
        drift.source.observe(AudioClockSample(1, 96048, 2_000_000_001), 6000)
        drift.destination.observe(AudioClockSample(2, 96000, 902_000_000_001), 6000)
        for (now in listOf(Long.MIN_VALUE, -1L, 1999L, 5001L, Long.MAX_VALUE)) assertNull(drift.correction(now))
        drift.source.observe(null, 2000)
        assertNull(drift.correction(2000))
        val destinationMissing = completed()
        assertEquals(500, destinationMissing.correction(2000))
        destinationMissing.destination.observe(null, 2000)
        assertNull(destinationMissing.correction(2000))
        drift.clear()
        assertNull(drift.correction(2000))
    }

    @Test fun t796_oneSidedWindowsAndEpochResetsDoNotReuseOldCorrection() {
        val drift = completed()
        assertEquals(500, drift.correction(2000))
        drift.source.observe(AudioClockSample(1, 192048, 4_000_000_001), 4000)
        assertEquals(0, drift.correction(4000))
        drift.destination.observe(AudioClockSample(2, 191952, 904_000_000_001), 4000)
        assertEquals(500, drift.correction(4000))
        assertTrue(drift.source.observe(AudioClockSample(3, 0, 1), 4000))
        assertNull(drift.correction(4000))
        drift.source.observe(AudioClockSample(3, 95952, 2_000_000_001), 6000)
        assertEquals(0, drift.correction(6000))
        drift.clear()
        assertNull(drift.correction(6000))
        drift.source.observe(AudioClockSample(7, 0, 1), 6000)
        drift.destination.observe(AudioClockSample(8, 0, 1), 6000)
        drift.source.observe(AudioClockSample(7, 96000, 2_000_000_001), 8000)
        drift.destination.observe(AudioClockSample(8, 96000, 2_000_000_001), 8000)
        assertEquals(0, drift.correction(8000))
    }

    @Test fun t796_repeatedOutOfRangeRatesAlwaysRejectAndExactBoundsRemainValid() {
        for (extra in -98L..98L) {
            val drift = completed(96000 + extra)
            repeat(3) {
                if (extra in -96..96) assertEquals((extra * 1_000_000 / 96000).toInt(), drift.correction(2000))
                else assertThrows(IllegalArgumentException::class.java) { drift.correction(2000) }
            }
            assertNull(drift.correction(5001))
        }
    }
}
