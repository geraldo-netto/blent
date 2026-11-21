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

    @Test fun t1565IdleBurstReceivesItsOwnOutputObservationWindow() {
        for (start in listOf(0L, -60_000_000_000L, Long.MAX_VALUE - 1_000_000_000L)) {
            val watchdog = DecoderOutputWatchdog()
            watchdog.restarted(start)
            var now = start
            val random = java.util.Random(1565)
            repeat(64) {
                now += 2_000_000_000L + random.nextInt(60_000).toLong() * 1_000_000
                repeat(4) { frame ->
                    assertNull("T1565 idle time manufactured a decoder stall", watchdog.queued(now + frame))
                }
                watchdog.output(now + 4)
                assertTrue("T1565 idle bursts disabled hints", watchdog.lowLatencyHints)
            }
        }
    }

    @Test fun t1565UnansweredBurstStillExpiresAtItsOwnBoundary() {
        val watchdog = DecoderOutputWatchdog()
        watchdog.restarted(0)
        val first = 60_000_000_000L
        repeat(3) { assertNull(watchdog.queued(first + it)) }
        assertNull("T1565 inclusive watchdog boundary", watchdog.queued(first + 1_500_000_000L))
        val stall = checkNotNull(watchdog.queued(first + 1_500_000_001L))
        assertEquals(5, stall.queued)
        assertEquals(1_500_000_001L, stall.silentNanos)
        assertFalse(stall.droppedHints)
    }

    @Test fun t1107_sparsePolicyCannotHideAdmittedUncompletedPictures() {
        val watchdog = DecoderOutputWatchdog()
        watchdog.restarted(0)
        repeat(3) { assertNull(watchdog.queued(2_000_000_000L + it)) }
        val stalled = watchdog.queued(3_500_000_001L)!!
        assertEquals(4, stalled.queued)
        assertEquals(1_500_000_001L, stalled.silentNanos)
        watchdog.output(4_000_000_000L)
        assertNull(watchdog.queued(63_000_000_000L))
    }
}
