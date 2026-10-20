package com.blent

import org.junit.Assert.*
import org.junit.Test
import java.math.BigInteger
import java.util.concurrent.CopyOnWriteArrayList
import java.util.concurrent.CountDownLatch

/** T1180: bounded independent callback-authority transition oracle. */
class GenerationFenceTest {
    @Test fun t1180DefaultStartsClosedAndOnlyFreshGrantBecomesCurrent() {
        val fence = GenerationFence()
        assertFalse(fence.current(0))
        assertEquals(1L, fence.acquire())
        assertTrue(fence.current(1))
        fence.revoke()
        assertFalse(fence.current(1))
        assertEquals(2L, fence.acquire())
        assertFalse(fence.current(1))
        assertTrue(fence.current(2))
    }

    @Test fun t1180NegativeSeedsNeverCreateAuthority() {
        for (seed in (Long.MIN_VALUE..Long.MIN_VALUE + 16) + (-16L..-1L)) {
            assertThrows(IllegalArgumentException::class.java) { GenerationFence(seed) }
        }
    }

    @Test fun t1180ExhaustiveBoundedTracesMatchWideIndependentAuthorityModel() {
        for (seed in (0L..16L) + (Long.MAX_VALUE - 16..Long.MAX_VALUE)) {
            repeat(81) { trace -> checkTrace(seed, trace) }
        }
    }

    private fun checkTrace(seed: Long, trace: Int) {
        val fence = GenerationFence(seed)
        val maximum = BigInteger.valueOf(Long.MAX_VALUE)
        var expected = BigInteger.valueOf(seed)
        var active = false
        var remaining = trace
        checkState(fence, expected, active)
        repeat(4) {
            val action = remaining % 3
            remaining /= 3
            when (action) {
                0 -> {
                    active = expected < maximum
                    if (active) expected += BigInteger.ONE
                    assertEquals(if (active) expected.toLong() else null, fence.acquire())
                }
                1 -> { active = false; fence.revoke() }
                2 -> Unit
            }
            checkState(fence, expected, active)
        }
    }

    private fun checkState(fence: GenerationFence, expected: BigInteger, active: Boolean) {
        assertEquals(expected < BigInteger.valueOf(Long.MAX_VALUE), fence.available())
        val probes = listOf(Long.MIN_VALUE, -1, 0, 1, Long.MAX_VALUE,
            expected.toLong(), (expected - BigInteger.ONE).toLong())
        for (value in probes) {
            assertEquals(active && BigInteger.valueOf(value) == expected, fence.current(value))
        }
    }

    @Test fun t1180ConcurrentTerminalReplacementNeverRevivesOriginalGrant() {
        val fence = GenerationFence(Long.MAX_VALUE - 1)
        val gate = CountDownLatch(1)
        val outcomes = CopyOnWriteArrayList<Long?>()
        val threads = ArrayList<Thread>(8)
        try {
            repeat(8) {
                threads += Thread { gate.await(); outcomes.add(fence.acquire()) }
                threads.last().start()
            }
        } finally {
            gate.countDown()
            threads.forEach { it.join() }
        }
        assertEquals(8, outcomes.size)
        assertEquals(listOf(Long.MAX_VALUE), outcomes.filterNotNull())
        assertFalse(fence.current(Long.MAX_VALUE))
        repeat(3) { fence.revoke(); assertNull(fence.acquire()) }
        assertFalse(fence.current(Long.MAX_VALUE))
    }
}
