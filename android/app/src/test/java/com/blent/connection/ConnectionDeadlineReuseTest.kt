package com.blent.connection

import org.junit.Assert.*
import org.junit.Test

class ConnectionDeadlineReuseTest {
    @Test fun t1097IdleProjectionReusesItsImmutableSnapshot() {
        repeat(128) {
            assertSame(ConnectionDeadline.IDLE, ConnectionDeadline.at(null))
            assertSame(ConnectionDeadline.IDLE, ConnectionDeadline.at(null).combine(ConnectionDeadline.IDLE))
            assertSame(ConnectionDeadline.UNKNOWN, ConnectionDeadline.IDLE.combine(ConnectionDeadline.UNKNOWN))
        }
    }

    @Test fun t1097BoundedOracleReusesAnInputExactlyWhenItsValueMatches() {
        val times = listOf(null, 0L, 1L, 31L, Long.MAX_VALUE - 1, Long.MAX_VALUE)
        val snapshots = times.flatMap { time -> listOf(ConnectionDeadline(time, false), ConnectionDeadline(time, true)) }
        for (first in snapshots) for (second in snapshots) {
            val inputs = listOf(first, second)
            val expected = ConnectionDeadline(inputs.mapNotNull { it.atMs }.sorted().firstOrNull(), inputs.all { it.known })
            val actual = first.combine(second)
            assertEquals(expected, actual)
            if (expected == first) assertSame(first, actual)
            else if (expected == second) assertSame(second, actual)
        }
    }

    @Test fun t1097NewPartialSnapshotPreservesBothOriginalValuesAndMaxDeadline() {
        val original = ConnectionDeadline.at(Long.MAX_VALUE)
        val uncertain = ConnectionDeadline.UNKNOWN
        val result = original.combine(uncertain)
        assertEquals(ConnectionDeadline(Long.MAX_VALUE, false), result)
        assertNotSame(original, result)
        assertNotSame(uncertain, result)
        assertEquals(ConnectionDeadline(Long.MAX_VALUE, true), original)
        assertEquals(ConnectionDeadline(null, false), uncertain)
        assertSame(result, result.combine(uncertain))
        assertSame(result, result.combine(ConnectionDeadline.IDLE))
    }
}
