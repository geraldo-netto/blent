package com.blent

import kotlinx.coroutines.*
import org.junit.Assert.*
import org.junit.Test

class ResourceRetirementErrorTest {
    private class Run(private val failure: Throwable? = null) : LifecycleRun {
        var started = 0
        var retirementCalls = 0
        override suspend fun execute() { started++; awaitCancellation() }
        override fun cancel() {}
        override suspend fun retire() { retirementCalls++; failure?.let { throw it } }
    }
    private fun scope(errors: MutableList<Throwable>) = CoroutineScope(
        SupervisorJob() + Dispatchers.Unconfined + CoroutineExceptionHandler { _, error -> errors.add(error) })

    private fun assertPropagation(original: Throwable, observed: Throwable) {
        assertEquals(original.javaClass, observed.javaClass)
        assertEquals(original.message, observed.message)
        val provenance = generateSequence(observed) { it.cause }.take(8).toList()
        assertTrue("T947 coroutine recovery must retain original fatal provenance", provenance.any { it === original })
    }

    @Test fun t947EndedErrorStillRetiresNativeOwnerBeforeReplacementAndPropagates() = runBlocking {
        val fatal = AssertionError("synthetic observer failure")
        val errors = mutableListOf<Throwable>()
        val scope = scope(errors)
        val lane = ResourceLane(scope, {})
        val first = Run()
        val second = Run()
        val proof = mutableListOf<ResourceRetirement>()
        try {
            lane.replace({ first }, { throw fatal }, proof::add)
            lane.replace({ second }, {})
            assertEquals("T947 Error cannot skip native retirement", 1, first.retirementCalls)
            assertEquals(listOf(ResourceRetirement.RETIRED), proof)
            assertEquals(1, second.started)
            assertEquals(1, errors.size)
            assertSame("T947 fatal observer Error is propagated unchanged", fatal, errors.single())
        } finally { lane.stop(); scope.cancel(); scope.coroutineContext[Job]!!.join() }
    }

    @Test fun t947NativeErrorsRetainOriginalOwnerAndCannotGrantProofOrReplacement() = runBlocking {
        for (fatal in listOf(AssertionError("synthetic release"), OutOfMemoryError("synthetic allocation"), LinkageError("synthetic linkage"))) {
            val errors = mutableListOf<Throwable>()
            val scope = scope(errors)
            val lane = ResourceLane(scope, {})
            val other = ResourceLane(scope, {})
            val failed = Run(fatal)
            val independent = Run()
            val proof = mutableListOf<ResourceRetirement>()
            val unopened = mutableListOf<ResourceRetirement>()
            var constructed = 0
            try {
                lane.replace({ failed }, {}, proof::add)
                lane.replace({ constructed++; Run() }, {}, unopened::add)
                assertEquals(1, failed.retirementCalls)
                assertTrue("T947 failed native release cannot grant proof", proof.isEmpty())
                assertNotNull("T947 Error must quarantine native owner", lane.error)
                assertSame(fatal, lane.error!!.cause)
                assertPropagation(fatal, errors.single())
                assertEquals(0, constructed)
                assertEquals(listOf(ResourceRetirement.NOT_STARTED), unopened)
                assertTrue(runCatching { lane.replace({ constructed++; Run() }, {}) }.isFailure)
                assertEquals(0, constructed)
                other.replace({ independent }, {})
                assertEquals(1, independent.started)
            } finally { lane.stop(); other.stop(); scope.cancel(); scope.coroutineContext[Job]!!.join() }
        }
    }

    @Test fun t947DirectGuardPreservesFirstFailureAndPropagatesOriginalThrowable() = runBlocking {
        val guard = RetirementGuard()
        val first = IllegalStateException("original native failure")
        assertSame(first, guard.retire(Run(first)))
        val fatal = Throwable("synthetic direct throwable")
        val empty = RetirementGuard()
        assertSame(fatal, runCatching { empty.retire(Run(fatal)) }.exceptionOrNull())
        assertSame(fatal, empty.error?.cause)
        assertTrue(runCatching { empty.checkAvailable() }.isFailure)
        assertSame(fatal, runCatching { guard.retire(Run(fatal)) }.exceptionOrNull())
        assertSame("T947 later failures cannot erase retained ownership", first, guard.error)
        assertTrue(runCatching { guard.checkAvailable() }.isFailure)
    }
}
