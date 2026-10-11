package com.blent

import kotlinx.coroutines.*
import org.junit.Assert.*
import org.junit.Test
import java.util.concurrent.ConcurrentLinkedQueue

class ResourceRetirementNotificationTest {
    private class Run : LifecycleRun {
        val began = java.util.concurrent.CountDownLatch(1)
        @Volatile var started = 0
        var retired = 0
        var failure: Exception? = null
        val release = CompletableDeferred<Unit>()
        override suspend fun execute() { started++; began.countDown(); awaitCancellation() }
        override fun cancel() {}
        override suspend fun retire() { release.await(); failure?.let { throw it }; retired++ }
    }
    private fun scope(errors: ConcurrentLinkedQueue<Throwable> = ConcurrentLinkedQueue()) =
        CoroutineScope(SupervisorJob() + Dispatchers.Unconfined + CoroutineExceptionHandler { _, error -> errors.add(error) })

    @Test fun t942ProofFollowsActualRetirementBeforeReplacement() = runBlocking {
        val scope = scope()
        val lane = ResourceLane(scope, {})
        val first = Run()
        val second = Run().apply { release.complete(Unit) }
        val events = mutableListOf<String>()
        try {
            lane.replace({ first }, { events.add("ended") }, { proof ->
                assertEquals(ResourceRetirement.RETIRED, proof)
                assertEquals(1, first.retired)
                events.add("retired")
            })
            lane.replace({ events.add("next"); second }, {})
            assertEquals(listOf("ended"), events)
            assertEquals(0, second.started)
            first.release.complete(Unit)
            assertEquals(listOf("ended", "retired", "next"), events)
            assertEquals(1, second.started)
        } finally { first.release.complete(Unit); lane.stop(); scope.cancel(); scope.coroutineContext[Job]!!.join() }
    }

    @Test fun t942FailedReleaseNeverPublishesProofAndUnopenedPendingIsRetired() = runBlocking {
        val scope = scope()
        val lane = ResourceLane(scope, {})
        val first = Run().apply { failure = IllegalStateException("native stuck") }
        val opened = mutableListOf<ResourceRetirement>()
        val unopened = mutableListOf<ResourceRetirement>()
        try {
            lane.replace({ first }, {}, opened::add)
            lane.replace({ error("quarantined replacement must not open") }, {}, unopened::add)
            first.release.complete(Unit)
            assertEquals("native stuck", lane.error?.message)
            assertTrue(opened.isEmpty())
            assertEquals(listOf(ResourceRetirement.NOT_STARTED), unopened)
            lane.stop(); assertTrue(opened.isEmpty())
        } finally { first.release.complete(Unit); scope.cancel(); scope.coroutineContext[Job]!!.join() }
    }

    @Test fun t942CancellationBeforeDispatchWaitsForActualJobCompletion() = runBlocking {
        val queue = java.util.ArrayDeque<Runnable>()
        val dispatcher = object : CoroutineDispatcher() {
            override fun dispatch(context: kotlin.coroutines.CoroutineContext, block: Runnable) { queue.add(block) }
        }
        val scope = CoroutineScope(SupervisorJob() + dispatcher)
        val lane = ResourceLane(scope, {})
        val proofs = mutableListOf<ResourceRetirement>()
        var factories = 0
        lane.replace({ factories++; Run() }, {}, proofs::add)
        lane.stop()
        assertTrue("T942 queued job still owns original request", proofs.isEmpty())
        while (queue.isNotEmpty()) queue.removeFirst().run()
        assertEquals(0, factories)
        assertEquals(listOf(ResourceRetirement.NOT_STARTED), proofs)
        scope.cancel()
        while (queue.isNotEmpty()) queue.removeFirst().run()
        assertEquals(1, proofs.size)
    }

    @Test fun t942DiscardedPendingAndInactiveRequestsPublishNoStartExactlyOnce() = runBlocking {
        val scope = scope()
        val lane = ResourceLane(scope, {})
        val first = Run()
        val proofs = mutableListOf<String>()
        fun pending(name: String) = lane.replace({ error("discarded request opened") }, {}, {
            assertEquals(ResourceRetirement.NOT_STARTED, it); proofs.add(name)
        })
        lane.replace({ first }, {}, { proofs.add("owner") })
        pending("superseded"); pending("stopped"); lane.stop(); lane.stop()
        assertEquals(listOf("superseded", "stopped"), proofs)
        first.release.complete(Unit)
        assertEquals(listOf("superseded", "stopped", "owner"), proofs)
        scope.cancel(); scope.coroutineContext[Job]!!.join()
        pending("inactive")
        assertEquals(listOf("superseded", "stopped", "owner", "inactive"), proofs)
    }

    @Test fun t942PureFactoryFailureReportsNoStartAndPreservesAdmission() = runBlocking {
        val scope = scope()
        val lane = ResourceLane(scope, {})
        val proofs = mutableListOf<ResourceRetirement>()
        var failure: Exception? = null
        lane.replace({ error("pure factory failed") }, { failure = it }, proofs::add)
        assertEquals("pure factory failed", failure?.message)
        assertEquals(listOf(ResourceRetirement.NOT_STARTED), proofs)
        val next = Run().apply { release.complete(Unit) }
        lane.replace({ next }, {}, proofs::add)
        lane.stop()
        assertEquals(listOf(ResourceRetirement.NOT_STARTED, ResourceRetirement.RETIRED), proofs)
        assertEquals(1, next.retired)
        scope.cancel(); scope.coroutineContext[Job]!!.join()
    }

    @Test fun t942ProofAndReportCallbackFailureCannotLoseNextOwner() = runBlocking {
        val errors = ConcurrentLinkedQueue<Throwable>()
        val scope = scope(errors)
        val reports = mutableListOf<String>()
        val lane = ResourceLane(scope, { reports.add(it); error("observer failed") })
        val first = Run().apply { release.complete(Unit) }
        val second = Run().apply { release.complete(Unit) }
        var calls = 0
        try {
            lane.replace({ first }, {}, { calls++; error("proof observer failed") })
            lane.replace({ second }, {})
            assertEquals(1, first.retired)
            assertEquals(1, second.started)
            assertEquals(1, calls)
            assertTrue(errors.isEmpty())
            assertTrue(reports.any { it.contains("proof observer failed") })
        } finally { lane.stop(); scope.cancel(); scope.coroutineContext[Job]!!.join() }
    }

    @Test fun t942ReentrantProofCannotStartTwoReplacementsOrReleaseLaterOwner() = runBlocking {
        val scope = scope()
        val lane = ResourceLane(scope, {})
        val first = Run()
        val discarded = Run().apply { release.complete(Unit) }
        val latest = Run().apply { release.complete(Unit) }
        val abandoned = mutableListOf<ResourceRetirement>()
        try {
            lane.replace({ first }, {}, { lane.replace({ latest }, {}) })
            lane.replace({ discarded }, {}, abandoned::add)
            first.release.complete(Unit)
            assertEquals(0, discarded.started)
            assertEquals(listOf(ResourceRetirement.NOT_STARTED), abandoned)
            assertEquals(1, latest.started)
            lane.stop(); assertEquals(1, latest.retired)
        } finally { first.release.complete(Unit); lane.stop(); scope.cancel(); scope.coroutineContext[Job]!!.join() }
    }

    @Test fun t942RecoveryObserverCannotAdmitOverRetiringReplacement() = runBlocking {
        val scope = CoroutineScope(SupervisorJob() + Dispatchers.Default)
        val first = Run()
        val second = Run()
        val third = Run().apply { release.complete(Unit) }
        var observed = false
        val reports = ConcurrentLinkedQueue<String>()
        val observedCompletion = CompletableDeferred<Unit>()
        lateinit var lane: ResourceLane
        lane = ResourceLane(scope, { text ->
            reports.add(text)
            if (text == "Native cleanup complete." && !observed) {
                observed = true
                lane.replace({ second }, {})
                check(second.began.await(2, java.util.concurrent.TimeUnit.SECONDS))
                lane.replace({ third }, {})
                observedCompletion.complete(Unit)
            }
        }, 1)
        try {
            lane.replace({ first }, {})
            withContext(Dispatchers.IO) { check(first.began.await(2, java.util.concurrent.TimeUnit.SECONDS)) }
            val original = scope.coroutineContext[Job]!!.children.single()
            lane.stop()
            withTimeout(2000) { while (!lane.stalled) delay(1) }
            first.release.complete(Unit)
            withTimeout(2000) { original.join(); observedCompletion.await() }
            // join may return after Job state completion but before its native
            // completion handler finishes. Wait for that serialized handler too.
            withContext(Dispatchers.IO) { synchronized(lane) {} }
            assertEquals("T942 recovery reports=$reports first retired=${first.retired} error=${lane.error}", 1, second.started)
            assertEquals("T942 reporting cannot admit over a retained owner", 0, third.started)
            assertEquals("T942 only the retiring replacement owns a job", 1, scope.coroutineContext[Job]!!.children.count())
            second.release.complete(Unit)
            withContext(Dispatchers.IO) { check(third.began.await(2, java.util.concurrent.TimeUnit.SECONDS)) }
            assertEquals(1, third.started)
        } finally {
            first.release.complete(Unit); second.release.complete(Unit)
            lane.stop(); scope.cancel(); withTimeout(2000) { scope.coroutineContext[Job]!!.join() }
        }
    }
    @Test fun t942BoundedReplacementAndStopSequencesNeverDuplicateOrLoseNoStartProof() = runBlocking {
        val scope = scope()
        val lane = ResourceLane(scope, {})
        val owner = Run()
        val counts = mutableMapOf<Int, Int>()
        val expected = mutableSetOf<Int>()
        val random = java.util.Random(942)
        var closed = 0
        try {
            lane.replace({ owner }, {}, { proof -> assertEquals(ResourceRetirement.RETIRED, proof); closed++ })
            repeat(256) { index ->
                if (random.nextBoolean()) lane.stop()
                else {
                    expected.add(index)
                    lane.replace({ error("pending request constructed during retirement") }, {}, { proof ->
                        assertEquals(ResourceRetirement.NOT_STARTED, proof)
                        counts[index] = (counts[index] ?: 0) + 1
                    })
                }
            }
            lane.stop()
            assertEquals(expected, counts.keys)
            assertTrue(counts.values.all { it == 1 })
            assertEquals(0, closed)
            owner.release.complete(Unit)
            assertEquals(1, closed)
            assertEquals(1, owner.retired)
        } finally { owner.release.complete(Unit); lane.stop(); scope.cancel(); scope.coroutineContext[Job]!!.join() }
    }

}
