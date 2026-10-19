package com.blent.connection

import org.junit.Assert.*
import org.junit.Test

class ConnectionWaitTest {
    @Test fun t918MeasuredIdleDefaultPreservesDeadlineAndRetirementBounds() {
        val limits = ConnectionWorkerSpec("fixture").waitLimits()
        assertEquals(100, limits.fallbackMs)
        assertEquals(100, ConnectionWait.fallback(limits, false))
        assertEquals(10, ConnectionWait.fallback(limits, true))
        assertEquals(10, ConnectionWait.delayMs(0, ConnectionDeadline.UNKNOWN, limits.fallbackMs))
        assertEquals(2, ConnectionWait.delayMs(5, ConnectionDeadline.at(7), limits.fallbackMs))
    }

    @Test fun t1092UnknownFallbackAndKnownDeadlineAreIndependentBounds() {
        assertEquals(1000, ConnectionWait.delayMs(0, ConnectionDeadline.IDLE, 1000))
        assertEquals(10, ConnectionWait.delayMs(0, ConnectionDeadline.UNKNOWN, 1000))
        assertEquals(5, ConnectionWait.delayMs(0, ConnectionDeadline(5, false), 1000))
        assertEquals(1, ConnectionWait.delayMs(Long.MAX_VALUE - 1, ConnectionDeadline.at(Long.MAX_VALUE), 1000))
        assertEquals(0, ConnectionWait.delayMs(Long.MAX_VALUE, ConnectionDeadline.at(Long.MAX_VALUE), 1000))
    }

    @Test fun t1092BoundedWaitOraclePreservesEveryKnownAndFallbackLimit() {
        for (now in listOf(0L, 1L, 5L, Long.MAX_VALUE - 10, Long.MAX_VALUE))
            for (due in listOf(null, 0L, 1L, 5L, 10L, Long.MAX_VALUE))
                for (known in listOf(false, true)) for (fallback in listOf(1L, 5L, 10L, 1000L)) {
                    val bounds = arrayListOf(fallback)
                    if (!known) bounds.add(10)
                    if (due != null) bounds.add(if (due <= now) 0 else due - now)
                    assertEquals(bounds.sorted().first(), ConnectionWait.delayMs(now, ConnectionDeadline(due, known), fallback))
                }
    }

    @Test fun t1092ConfiguredAsyncBudgetsAndRetirementCapLostHints() {
        val original = ConnectionWorkerSpec("fixture", timerMs = 1000)
        assertEquals(1000, ConnectionWait.fallback(original.waitLimits(), false))
        assertEquals(10, ConnectionWait.fallback(original.waitLimits(), true))
        val short = listOf(original.copy(nativeTimeoutMs = 1), original.copy(approvalTimeoutMs = 1),
            original.copy(io = ConnectionRunLimits(1, 5000)), original.copy(io = ConnectionRunLimits(5000, 1)),
            original.copy(session = ConnectionSessionLimits(resourceTimeoutMs = 1)))
        for (spec in short) {
            assertEquals(1, ConnectionWait.fallback(spec.waitLimits(), false))
            assertEquals(1, ConnectionWait.fallback(spec.waitLimits(), true))
        }
        assertEquals(100, ConnectionWorkerSpec("fixture").waitLimits().fallbackMs)
    }

    @Test fun t1092InvalidBoundsRejectBeforeSchedulingOrArithmetic() {
        val random = java.util.Random(1092)
        val negative = List(64) { random.nextLong() or Long.MIN_VALUE }
        val oversizedFallback = List(64) { 1001L + random.nextInt(Int.MAX_VALUE) }
        for (value in listOf(Long.MIN_VALUE, -1L, 0L, 1001L, Long.MAX_VALUE) + negative + oversizedFallback) {
            assertThrows(IllegalArgumentException::class.java) { ConnectionWaitLimits(value, 1) }
            assertThrows(IllegalArgumentException::class.java) { ConnectionWait.delayMs(0, ConnectionDeadline.IDLE, value) }
        }
        val oversizedBudget = List(64) { 600001L + random.nextInt(Int.MAX_VALUE) }
        for (value in listOf(Long.MIN_VALUE, -1L, 0L, 600001L, Long.MAX_VALUE) + negative + oversizedBudget) {
            assertThrows(IllegalArgumentException::class.java) { ConnectionWaitLimits(1, value) }
        }
        for (now in listOf(Long.MIN_VALUE, -1L) + negative) {
            assertThrows(IllegalArgumentException::class.java) { ConnectionWait.delayMs(now, ConnectionDeadline.IDLE, 1) }
        }
    }
}
