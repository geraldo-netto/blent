package com.blent.connection

import org.junit.Assert.*
import org.junit.Test

/** T1093: diagnostic budgets are independent of resource/native completion. */
class ConnectionDeadlineTest {
    @Test fun t1093UnknownProviderPreservesOtherKnownAbsoluteBudgets() {
        val known = ConnectionDeadline.at(5)
        assertEquals(ConnectionDeadline(5, false), known.combine(ConnectionDeadline.UNKNOWN))
        assertEquals(ConnectionDeadline(5, false), ConnectionDeadline.UNKNOWN.combine(known))
        assertEquals(known, known.combine(ConnectionDeadline.IDLE))
        assertEquals(ConnectionDeadline.UNKNOWN, ConnectionDeadline.observe(null))
        var current = known
        val source = object : ConnectionDeadlineSource {
            override fun nextDeadline(): ConnectionDeadline = current
        }
        val original = ConnectionDeadline.observe(source)
        current = ConnectionDeadline.at(1)
        assertEquals(ConnectionDeadline.at(5), original)
        assertEquals(current, ConnectionDeadline.observe(source))
        assertEquals(ConnectionDeadline.IDLE, ConnectionDeadline.at(null))
    }

    @Test fun t1093BoundedCompositionMatchesIndependentSortedBudgetOracle() {
        val times = listOf(null, 0L, 1L, 31L, Long.MAX_VALUE - 1, Long.MAX_VALUE)
        val snapshots = times.flatMap { time -> listOf(ConnectionDeadline(time, false), ConnectionDeadline(time, true)) }
        for (first in snapshots) for (second in snapshots) for (third in snapshots) {
            val inputs = listOf(first, second, third)
            val expected = ConnectionDeadline(inputs.mapNotNull { it.atMs }.sorted().firstOrNull(), inputs.all { it.known })
            assertEquals(expected, first.combine(second).combine(third))
            assertEquals(expected, third.combine(second.combine(first)))
        }
    }

    @Test fun t1093NegativeAbsoluteTimesRejectWithoutOverflowSentinels() {
        val random = java.util.Random(1093)
        val invalid = listOf(Long.MIN_VALUE, -1L) + List(128) { random.nextLong() or Long.MIN_VALUE }
        for (time in invalid) {
            assertThrows(IllegalArgumentException::class.java) { ConnectionDeadline.at(time) }
        }
        assertEquals(0L, ConnectionDeadline.at(0).atMs)
        assertEquals(Long.MAX_VALUE, ConnectionDeadline.at(Long.MAX_VALUE).atMs)
    }
}
