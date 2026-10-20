package com.blent

import org.junit.Assert.*
import org.junit.Test
import java.math.BigInteger
import java.util.concurrent.CopyOnWriteArrayList
import java.util.concurrent.CountDownLatch

/** T1178: independent wide-number and finite concurrent admission oracles. */
class WorkerGenerationsTest {
    @Test fun t1178InvalidSeedsNeverCreateAuthority() {
        for (initial in (Long.MIN_VALUE..Long.MIN_VALUE + 16) + (-16L..-1L)) {
            assertThrows(IllegalArgumentException::class.java) { WorkerGenerations(initial) }
        }
    }

    @Test fun t1178BoundedWideOracleNeverWrapsOrReusesAnEpoch() {
        for (initial in (0L..16L) + (Long.MAX_VALUE - 16..Long.MAX_VALUE)) {
            checkSeed(initial)
        }
    }

    private fun checkSeed(initial: Long) {
        val generations = WorkerGenerations(initial)
        val maximum = BigInteger.valueOf(Long.MAX_VALUE)
        var expected = BigInteger.valueOf(initial)
        repeat(20) {
            val next = runCatching { generations.next() }
            if (expected < maximum) {
                expected += BigInteger.ONE
                assertEquals(expected, BigInteger.valueOf(next.getOrThrow()))
            } else {
                assertTrue(next.exceptionOrNull() is IllegalStateException)
            }
        }
    }

    @Test fun t1178ConcurrentLastAdmissionsAreUniqueAndAllThreadsJoin() {
        for (initial in listOf(0L, Long.MAX_VALUE - 8, Long.MAX_VALUE - 1, Long.MAX_VALUE)) {
            checkConcurrent(initial)
        }
    }

    private fun checkConcurrent(initial: Long) {
        val generations = WorkerGenerations(initial)
        val gate = CountDownLatch(1)
        val outcomes = CopyOnWriteArrayList<Result<Long>>()
        val threads = ArrayList<Thread>(8)
        try {
            repeat(8) {
                threads += Thread { gate.await(); outcomes.add(runCatching { generations.next() }) }
                threads.last().start()
            }
        } finally {
            gate.countDown()
            threads.forEach { it.join() }
        }
        val values = outcomes.mapNotNull { it.getOrNull() }.sorted()
        val expectedCount = (BigInteger.valueOf(Long.MAX_VALUE) - BigInteger.valueOf(initial))
            .min(BigInteger.valueOf(8)).toInt()
        assertEquals(expectedCount, values.size)
        assertEquals(values.size, values.toSet().size)
        assertEquals((1..expectedCount).map { BigInteger.valueOf(initial) + BigInteger.valueOf(it.toLong()) },
            values.map { BigInteger.valueOf(it) })
        assertTrue(outcomes.filter { it.isFailure }.all { it.exceptionOrNull() is IllegalStateException })
        assertEquals(8, outcomes.size)
    }
}
