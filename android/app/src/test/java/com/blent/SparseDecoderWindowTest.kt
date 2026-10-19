package com.blent

import org.junit.Assert.*
import org.junit.Test

/** T1107: only admitted pictures create decoder progress obligations. */
class SparseDecoderWindowTest {
    @Test fun t1107_sparseCompletedPicturesDoNotManufactureWatchdogStalls() {
        val watchdog = DecoderOutputWatchdog()
        watchdog.restarted(0)
        var now = 0L
        for (gap in listOf(1L, 1_500_000_000L, 1_500_000_001L, 60_000_000_000L)) {
            repeat(32) {
                now += gap
                assertNull("T1107 completed sparse picture treated as accumulated work", watchdog.queued(now))
                watchdog.output(now + 1)
                assertTrue(watchdog.lowLatencyHints)
            }
        }
    }

    @Test fun t1107_sparsePolicyCannotHideAdmittedUncompletedPictures() {
        val watchdog = DecoderOutputWatchdog()
        watchdog.restarted(0)
        repeat(3) { assertNull(watchdog.queued(2_000_000_000L + it)) }
        val stalled = watchdog.queued(2_000_000_003L)!!
        assertEquals(4, stalled.queued)
        assertEquals(2_000_000_003L, stalled.silentNanos)
        watchdog.output(3_000_000_000L)
        assertNull(watchdog.queued(63_000_000_000L))
    }
}
