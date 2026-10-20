package com.blent

import kotlinx.coroutines.*
import org.junit.Assert.*
import org.junit.Test
import java.math.BigInteger

/** T1190: observer exhaustion cannot forge or release native ownership. */
class NativeDeadlineTest {
    private fun timer(deadline: NativeDeadline): Job? =
        NativeDeadline::class.java.getDeclaredField("timer").apply { isAccessible = true }.get(deadline) as Job?
    private fun expire(deadline: NativeDeadline, revision: Long) {
        NativeDeadline::class.java.getDeclaredMethod("expired", Long::class.javaPrimitiveType)
            .apply { isAccessible = true }.invoke(deadline, revision)
    }
    @Test fun t1190TerminalBeginCannotAllocateTimerAndKeepsPendingStatusVisible() = runBlocking {
        val deadline = NativeDeadline("native", 60000, Long.MAX_VALUE)
        try {
            repeat(3) {
                deadline.begin()
                assertNull("T1190 terminal timer admitted", timer(deadline))
                assertTrue(deadline.status.value!!.contains("exhausted"))
                deadline.finish()
                assertNull(deadline.status.value)
            }
        } finally { val original = timer(deadline); deadline.finish(); original?.join() }
    }
    @Test fun t1190LastTimerFinishesWithoutAnotherIdentityAndCannotRestart() = runBlocking {
        val deadline = NativeDeadline("native", 60000, Long.MAX_VALUE - 1)
        var original: Job? = null
        try {
            deadline.begin(); original = checkNotNull(timer(deadline))
            deadline.begin(); assertSame(original, timer(deadline))
            expire(deadline, Long.MAX_VALUE)
            assertTrue(deadline.status.value!!.contains("stalled"))
            deadline.finish(); original.join()
            assertTrue(original.isCompleted)
            assertNull(deadline.status.value)
            deadline.begin()
            assertNull("T1190 exhausted observer restarted", timer(deadline))
            assertTrue(deadline.status.value!!.contains("exhausted"))
            val status = deadline.status.value
            expire(deadline, Long.MAX_VALUE)
            assertEquals(status, deadline.status.value)
            deadline.finish()
            expire(deadline, Long.MAX_VALUE)
            assertNull(deadline.status.value)
        } finally { val pending = timer(deadline); deadline.finish(); pending?.join(); original?.join() }
    }
    @Test fun t1190WrappedOrInvalidExpiryCannotPublishAnotherAttemptStatus() = runBlocking {
        val deadline = NativeDeadline("native", 60000, Long.MAX_VALUE)
        try {
            deadline.begin()
            val status = deadline.status.value
            for (revision in listOf(Long.MIN_VALUE, -1, 0, 1, Long.MAX_VALUE)) {
                expire(deadline, revision)
                assertEquals("T1190 stale expiry aliased current attempt", status, deadline.status.value)
            }
        } finally { val original = timer(deadline); deadline.finish(); original?.join() }
    }
    @Test fun t1190NegativeSeedsRejectBeforeTimerAdmission() {
        for (seed in (Long.MIN_VALUE..Long.MIN_VALUE + 16) + (-16L..-1L)) {
            assertThrows(IllegalArgumentException::class.java) { NativeDeadline("native", 60000, seed) }
        }
    }
    @Test fun t1190BoundedWideObserverModelKeepsOriginalJobsUntilCancellationJoins() = runBlocking {
        for (seed in (0L..16L) + (Long.MAX_VALUE - 16..Long.MAX_VALUE)) checkSeed(seed)
    }
    private suspend fun checkSeed(seed: Long) {
        val deadline = NativeDeadline("native", 60000, seed)
        val maximum = BigInteger.valueOf(Long.MAX_VALUE)
        var expected = BigInteger.valueOf(seed)
        try {
            repeat(20) {
                deadline.begin()
                val original = timer(deadline)
                if (expected < maximum) {
                    expected += BigInteger.ONE
                    assertNotNull(original)
                    assertEquals("native pending.", deadline.status.value)
                    expire(deadline, expected.toLong())
                    assertTrue(deadline.status.value!!.contains("stalled"))
                } else {
                    assertNull(original)
                    assertTrue(deadline.status.value!!.contains("exhausted"))
                }
                deadline.finish(); original?.join()
                expire(deadline, expected.toLong())
                assertNull(deadline.status.value)
                assertNull(timer(deadline))
            }
        } finally { val original = timer(deadline); deadline.finish(); original?.join() }
    }
}
