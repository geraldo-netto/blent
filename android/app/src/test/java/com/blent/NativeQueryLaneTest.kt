package com.blent

import kotlinx.coroutines.*
import org.junit.Assert.*
import org.junit.Test
import java.util.concurrent.*
import java.util.concurrent.atomic.AtomicInteger

class NativeQueryLaneTest {
    @Test fun t808_timeoutAndStormCannotReplaceNativeOwner() = runBlocking {
        val entered = CountDownLatch(1)
        val release = CountDownLatch(1)
        val starts = AtomicInteger()
        val lane = NativeQueryLane(30)
        try {
            val first = async(Dispatchers.Default) { lane.query { starts.incrementAndGet(); entered.countDown(); release.await(); 42 } }
            assertTrue(entered.await(2, TimeUnit.SECONDS))
            assertNull(first.await())
            repeat(200) { assertNull(lane.query { starts.incrementAndGet() }) }
            assertEquals(1, starts.get())
            assertTrue(lane.busy)
        } finally { release.countDown() }
        withTimeout(2_000) { while (lane.busy) delay(1) }
        assertEquals(43, lane.query { 43 })
        assertFalse(lane.busy)
    }

    @Test fun t808_cancellationDoesNotReleaseOwnerAndNoStaleValueReachesReplacement() = runBlocking {
        val entered = CountDownLatch(1)
        val release = CountDownLatch(1)
        val lane = NativeQueryLane()
        val old = async(Dispatchers.Default) { lane.query { entered.countDown(); release.await(); "old" } }
        try {
            assertTrue(entered.await(2, TimeUnit.SECONDS))
            old.cancelAndJoin()
            assertTrue(old.isCancelled)
            assertNull(lane.query { "premature" })
        } finally { release.countDown() }
        withTimeout(2_000) { while (lane.busy) delay(1) }
        assertEquals("new", lane.query { "new" })
    }

    @Test fun t808_failuresAndCancelledAdmissionNeverLeakWorkerBudget() = runBlocking {
        val lane = NativeQueryLane()
        assertNull(lane.query<Int> { error("native failure") })
        assertFalse(lane.busy)
        assertEquals(7, lane.query { 7 })
        val rejecting = NativeQueryLane(executor = Executor { throw RejectedExecutionException() })
        repeat(2) { assertNull(rejecting.query { 1 }); assertFalse(rejecting.busy) }
        var invoked = false
        val cancelled = launch(start = CoroutineStart.LAZY) { lane.query { invoked = true } }
        cancelled.cancel(); cancelled.join()
        assertFalse(invoked)
    }

    @Test fun t808_deadlineRangeRejectsInvalidValuesAndPreservesBounds() {
        for (value in listOf(Long.MIN_VALUE, -1L, 0L, 60_001L, Long.MAX_VALUE)) {
            assertThrows(IllegalArgumentException::class.java) { NativeQueryLane(value) }
        }
        for (value in listOf(1L, 60_000L)) assertFalse(NativeQueryLane(value).busy)
    }
}
