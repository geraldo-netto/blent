package com.blent.benchmark

import org.junit.Assert.*
import org.junit.Test

class ReplayRetirementTest {
    @Test fun t798_workerWaitsForConfirmedReleaseAndBoundsAStalledOwner() {
        var waits = 0
        awaitReplayRetirement({ true }, { waits < 3 }, { 0 }, { waits++ })
        assertEquals(3, waits)
        var now = 0L
        assertThrows(IllegalStateException::class.java) {
            awaitReplayRetirement({ true }, { true }, { now }, { now += 1_000_000_000 })
        }
        assertEquals(5_000_000_000L, now)
        assertThrows(IllegalStateException::class.java) {
            awaitReplayRetirement({ false }, { true }, { 0 }, { fail("cancelled replay waited") })
        }
        awaitReplayRetirement({ true }, { false }, { 0 }, { fail("retired owner waited") })
    }
}
