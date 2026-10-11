package com.blent

import kotlinx.coroutines.*
import org.junit.Assert.*
import org.junit.Test
import java.util.concurrent.ConcurrentLinkedQueue

class ResourceLaneTest {
    private class Run : LifecycleRun {
        var started = 0
        var cancelled = 0
        var retired = 0
        var cancelError: Exception? = null
        var retireError: Exception? = null
        val release = CompletableDeferred<Unit>()
        override suspend fun execute() { started++; awaitCancellation() }
        override fun cancel() { cancelled++; cancelError?.let { throw it } }
        override suspend fun retire() { release.await(); retired++; retireError?.let { throw it } }
    }

    @Test fun t806_timeoutReportsStallWithoutOpeningReplacementOrBlockingOtherLane() = runBlocking {
        val scope = CoroutineScope(SupervisorJob() + Dispatchers.Unconfined)
        val reports = ConcurrentLinkedQueue<String>()
        val lane = ResourceLane(scope, { reports.add(it) }, 10)
        val other = ResourceLane(scope, {})
        val first = Run()
        val last = Run()
        val independent = Run()
        try {
            lane.replace({ first }, {})
            first.cancelError = IllegalStateException("cancel failed")
            lane.replace({ last }, {})
            withTimeout(2000) { while (!lane.stalled) delay(1) }
            repeat(100) { lane.replace({ last }, {}) }
            assertTrue(reports.any { it.contains("cancel failed") })
            assertTrue(reports.any { it.contains("stalled") })
            assertEquals(1, first.cancelled)
            assertEquals(0, first.retired)
            assertEquals(0, last.started)
            other.replace({ independent }, {})
            assertEquals(1, independent.started)
            first.release.complete(Unit)
            assertEquals(1, first.retired)
            assertEquals(1, last.started)
            assertFalse(lane.stalled)
            assertTrue("T806 stale stall remained after retirement", reports.any { it.contains("cleanup complete") })
        } finally {
            first.release.complete(Unit); last.release.complete(Unit); independent.release.complete(Unit)
            lane.stop(); other.stop(); scope.cancel()
            withTimeout(2000) { scope.coroutineContext[Job]!!.join() }
        }
    }

    @Test fun t806_failedCleanupQuarantinesOwnerAndRejectsPendingFactory() = runBlocking {
        val scope = CoroutineScope(SupervisorJob() + Dispatchers.Unconfined)
        val reports = mutableListOf<String>()
        val lane = ResourceLane(scope, { reports.add(it) })
        val run = Run().apply { retireError = IllegalStateException("release failed") }
        var constructed = 0
        lane.replace({ run }, {})
        lane.replace({ constructed++; Run() }, {})
        run.release.complete(Unit)
        assertSame(run.retireError, lane.error)
        assertEquals(0, constructed)
        assertTrue(reports.any { it.contains("release failed") })
        assertTrue(runCatching { lane.replace({ Run() }, {}) }.isFailure)
        assertTrue(runCatching { lane.checkAvailable() }.isFailure)
        lane.stop(); scope.cancel()
    }

    @Test fun t806_factoryFailuresAndInactiveScopeDoNotStrandAdmission() = runBlocking {
        val scope = CoroutineScope(SupervisorJob() + Dispatchers.Unconfined)
        val lane = ResourceLane(scope, {})
        var failure: Exception? = null
        lane.replace({ error("factory failed") }, { failure = it })
        assertEquals("factory failed", failure?.message)
        val run = Run().apply { release.complete(Unit) }
        lane.replace({ run }, {})
        assertEquals(1, run.started)
        lane.stop()
        assertEquals(1, run.retired)
        scope.cancel()
        lane.replace({ error("inactive scope constructed native owner") }, {})
        lane.stop()
    }

    @Test fun t806_cancelBeforeDispatchNeverConstructsNativeOwner() = runBlocking {
        val queue = java.util.ArrayDeque<Runnable>()
        val dispatcher = object : CoroutineDispatcher() {
            override fun dispatch(context: kotlin.coroutines.CoroutineContext, block: Runnable) { queue.add(block) }
        }
        val scope = CoroutineScope(SupervisorJob() + dispatcher)
        val lane = ResourceLane(scope, {})
        var created = 0
        lane.replace({ created++; Run() }, {})
        lane.stop()
        while (queue.isNotEmpty()) queue.removeFirst().run()
        assertEquals(0, created)
        scope.cancel()
        assertFalse(lane.stalled)
    }

    @Test fun t942EndedCallbackFailureCannotSkipNativeRetirementOrStrandAdmission() = runBlocking {
        val errors = ConcurrentLinkedQueue<Throwable>()
        val scope = CoroutineScope(SupervisorJob() + Dispatchers.Unconfined + CoroutineExceptionHandler { _, error -> errors.add(error) })
        val reports = mutableListOf<String>()
        val lane = ResourceLane(scope, { reports.add(it) })
        val first = Run().apply { release.complete(Unit) }
        val second = Run().apply { release.complete(Unit) }
        try {
            lane.replace({ first }, { error("ended callback failed") })
            lane.replace({ second }, {})
            assertEquals("T942 ended callback cannot bypass native cleanup", 1, first.retired)
            assertEquals(1, second.started)
            assertTrue("T942 callback exception remains resource-local", errors.isEmpty())
            assertTrue(reports.any { it.contains("ended callback failed") })
        } finally { lane.stop(); scope.cancel(); scope.coroutineContext[Job]!!.join() }
    }

    @Test fun t806_deadlinesRejectInvalidRangesAndCannotClearLaterWait() = runBlocking {
        val scope = CoroutineScope(SupervisorJob() + Dispatchers.Unconfined)
        val invalid = listOf(Long.MIN_VALUE, -100, -1, 0, 60_001, Long.MAX_VALUE)
        for (value in invalid) {
            assertTrue(runCatching { ResourceLane(scope, {}, value) }.isFailure)
            assertTrue(runCatching { NativeDeadline("native", value) }.isFailure)
        }
        for (value in listOf(1L, 60_000L)) { ResourceLane(scope, {}, value); NativeDeadline("native", value) }
        val deadline = NativeDeadline("native", 10)
        deadline.begin(); deadline.begin()
        assertEquals("native pending.", deadline.status.value)
        withTimeout(2000) { while (deadline.status.value?.contains("stalled") != true) delay(1) }
        deadline.finish()
        assertNull(deadline.status.value)
        repeat(100) { deadline.begin(); deadline.finish() }
        deadline.begin()
        withTimeout(2000) { while (deadline.status.value?.contains("stalled") != true) delay(1) }
        deadline.finish(); scope.cancel()
    }
}
