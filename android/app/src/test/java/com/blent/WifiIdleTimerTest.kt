package com.blent

import org.junit.Assert.*
import org.junit.Test

class WifiIdleTimerTest {
    private fun target(owner: Any, due: Long, releases: MutableList<Any>, current: () -> Boolean = { true }) =
        WifiIdleTarget(owner, due, current) { releases.add(owner) }

    @Test fun t1060OldWakeHintsCannotReleaseNewOwnerEarlyOrAcknowledgeNewAdmission() {
        val clock = PowerRuntimeFixture.Schedule()
        val releases = ArrayList<Any>()
        val timer = WifiIdleTimer(clock) { throw it }
        val old = Any(); val fresh = Any()
        timer.set(target(old, 5000, releases))
        clock.time = 100
        timer.set(target(fresh, 5100, releases))
        timer.clear(old)
        assertEquals(1, clock.entries.size)
        val previous = clock.entries.single().action
        clock.time = 5000; clock.due()
        assertTrue(releases.isEmpty())
        assertEquals(5100L, clock.entries.single().due)
        previous.run()
        assertEquals(1, clock.entries.size)
        clock.time = 5100; clock.due()
        assertEquals(listOf(fresh), releases)
        assertTrue(clock.entries.isEmpty())
    }
    @Test fun t1060SuccessfulCancelFencesOldCallbackButUnknownCancelKeepsOriginalAdmission() {
        val clock = PowerRuntimeFixture.Schedule()
        val releases = ArrayList<Any>()
        val reports = ArrayList<Exception>()
        val timer = WifiIdleTimer(clock, reports::add)
        val old = Any(); val fresh = Any()
        timer.set(target(old, 5000, releases))
        val stale = clock.entries.single().action
        timer.clear(old)
        timer.set(target(fresh, 5000, releases))
        stale.run()
        assertEquals(1, clock.entries.size)
        clock.cancelFailure = true
        timer.clear(fresh)
        timer.set(target(old, 5000, releases))
        assertEquals(listOf(old), releases)
        assertEquals(1, clock.entries.size)
        assertEquals(1, reports.size)
        clock.time = 5000; clock.due()
        clock.cancelFailure = false
        timer.set(target(fresh, 10000, releases))
        assertEquals(1, clock.entries.size)
        timer.clear(fresh)
        assertTrue(clock.entries.isEmpty())
    }
    @Test fun t1060PostRejectionAndEnqueueThenThrowWithdrawGraceWithoutLosingOriginalNotice() {
        val clock = PowerRuntimeFixture.Schedule()
        val releases = ArrayList<Any>()
        val reportFailure = IllegalStateException("T1060 timer reporter failed")
        val reports = ArrayList<Exception>()
        val timer = WifiIdleTimer(clock) { reports.add(it); throw reportFailure }
        val owner = Any()
        clock.refuse = true
        timer.set(target(owner, 5000, releases))
        assertEquals(1, releases.size); assertTrue(clock.entries.isEmpty())
        clock.refuse = false; clock.postFailure = true
        timer.set(target(owner, 5000, releases))
        assertEquals(2, releases.size); assertEquals(1, clock.entries.size)
        assertTrue(reports.single().suppressed.contains(reportFailure))
        timer.set(target(owner, 5000, releases))
        assertEquals(3, releases.size); assertEquals(1, clock.entries.size)
        clock.time = 5000; clock.due()
        clock.postFailure = false
        timer.set(target(owner, 10000, releases))
        assertEquals(1, clock.entries.size)
        timer.clear(owner)
    }
    @Test fun t1060InvalidCallbackClockWithdrawsGraceAndReportsWithoutEscapingMainLoop() {
        val clock = PowerRuntimeFixture.Schedule()
        val releases = ArrayList<Any>()
        val reports = ArrayList<Exception>()
        val timer = WifiIdleTimer(clock, reports::add)
        val owner = Any()
        timer.set(target(owner, 5000, releases))
        val callback = clock.entries.single().action
        clock.entries.clear(); clock.time = -1
        assertTrue("T1060 invalid native clock escaped the main callback", runCatching { callback.run() }.isSuccess)
        assertEquals(listOf(owner), releases)
        assertEquals(1, reports.size)
        assertTrue(clock.entries.isEmpty())
        callback.run(); assertEquals(1, releases.size)
    }
    @Test fun t1060InvalidClockAndDeadlineBoundsNeverQueueAndStaleViewCannotRelease() {
        val clock = PowerRuntimeFixture.Schedule()
        val releases = ArrayList<Any>()
        val timer = WifiIdleTimer(clock) { throw it }
        val owner = Any()
        for (invalid in listOf(Long.MIN_VALUE, -1, 5001, Long.MAX_VALUE)) {
            assertTrue(runCatching { timer.set(target(owner, invalid, releases)) }.isFailure)
            assertTrue(clock.entries.isEmpty())
        }
        clock.time = -1
        assertTrue(runCatching { timer.set(target(owner, 1, releases)) }.isFailure)
        clock.time = Long.MAX_VALUE - 5000
        timer.set(target(owner, Long.MAX_VALUE, releases) { false })
        clock.time = Long.MAX_VALUE; clock.due()
        assertTrue(releases.isEmpty())
        timer.set(target(owner, Long.MAX_VALUE, releases))
        assertEquals(listOf(owner), releases)
    }
}
