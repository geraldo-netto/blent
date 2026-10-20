package com.blent.connection

import com.blent.LifecycleRun
import java.util.concurrent.Callable
import java.util.concurrent.Executors
import java.util.concurrent.TimeUnit
import kotlinx.coroutines.*
import kotlinx.coroutines.flow.take
import kotlinx.coroutines.flow.toList
import org.junit.Assert.*
import org.junit.Test

class ConnectionAccessTest {
    private class Run : LifecycleRun {
        val finish = CompletableDeferred<Unit>()
        val release = CompletableDeferred<Unit>()
        val retiring = CompletableDeferred<Unit>()
        var executions = 0
        var cancellations = 0
        var retirements = 0
        var failRetire = false
        override suspend fun execute() { executions++; finish.await() }
        override fun cancel() { cancellations++ }
        override suspend fun retire() { retirements++; retiring.complete(Unit); check(!failRetire); release.await() }
    }

    @Test fun t1167OnlyOriginalPermitCanReturnOneBoundedSlot() {
        val access = ConnectionAccess()
        assertNull(access.current())
        for (use in ConnectionUse.entries) {
            val original = access.acquire(use)
            assertEquals(use, access.current())
            for (other in ConnectionUse.entries) assertThrows(IllegalStateException::class.java) { access.acquire(other) }
            val forged = ConnectionAccess.Permit(access, use)
            assertThrows(IllegalStateException::class.java) { forged.close() }
            assertEquals(use, access.current())
            original.close()
            assertNull(access.current())
        }
    }

    @Test fun t1167StaleClosedPermitsNeverEraseReplacementIdentities() {
        val access = ConnectionAccess()
        val old = access.acquire(ConnectionUse.CONNECTION).also { it.close() }
        repeat(1024) {
            access.acquire(ConnectionUse.SAVED_TRUST).use {
                old.close()
                assertEquals(ConnectionUse.SAVED_TRUST, access.current())
            }
        }
        assertNull(access.current())
    }

    @Test fun t1167ConcurrentClaimsHaveExactlyOneOriginalOwner() {
        val access = ConnectionAccess()
        val pool = Executors.newFixedThreadPool(8)
        val permits = try {
            pool.invokeAll(List(32) { index -> Callable {
                runCatching { access.acquire(ConnectionUse.entries[index % 2]) }.getOrNull()
            } }).mapNotNull { it.get() }
        } finally {
            pool.shutdown()
            while (!pool.awaitTermination(1, TimeUnit.SECONDS)) { /* Actual original worker exit, without a runtime cap. */ }
        }
        try { assertEquals(1, permits.size) } finally { permits.forEach { it.close() } }
        assertNull(access.current())
    }

    @Test fun t1167ModeObservationsCarryNoPermitAndTrackBothUses() = runBlocking<Unit> {
        val access = ConnectionAccess()
        val observed = async(start = CoroutineStart.UNDISPATCHED) { access.changes.take(5).toList() }
        val first = access.acquire(ConnectionUse.CONNECTION); yield()
        first.close(); yield()
        val second = access.acquire(ConnectionUse.SAVED_TRUST); yield()
        second.close()
        assertEquals(listOf(null, ConnectionUse.CONNECTION, null, ConnectionUse.SAVED_TRUST, null), observed.await())
    }

    @Test fun t1167LogicalEndAndCancellationRetainSlotUntilOriginalRetirement() = runBlocking<Unit> {
        val access = ConnectionAccess(); val native = Run()
        val run = access.guard(ConnectionUse.CONNECTION, native)
        assertNull(access.current()); assertEquals(0, native.executions)
        native.finish.complete(Unit); run.execute()
        run.cancel(); run.cancel()
        assertEquals(1, native.cancellations)
        assertEquals(ConnectionUse.CONNECTION, access.current())
        val retiring = async(start = CoroutineStart.UNDISPATCHED) { run.retire() }
        assertFalse(retiring.isCompleted)
        assertThrows(IllegalStateException::class.java) { access.acquire(ConnectionUse.SAVED_TRUST) }
        native.release.complete(Unit); retiring.await()
        run.retire(); run.cancel()
        assertEquals(1, native.retirements); assertEquals(1, native.cancellations)
        assertNull(access.current())
        assertTrue(runCatching { run.execute() }.isFailure)
    }

    @Test fun t1167FailedRetirementKeepsOriginalIdentityUntilActualSuccessfulRetry() = runBlocking<Unit> {
        val access = ConnectionAccess(); val native = Run().apply { failRetire = true; finish.complete(Unit) }
        val run = access.guard(ConnectionUse.SAVED_TRUST, native)
        run.execute()
        assertTrue(runCatching { run.retire() }.isFailure)
        assertEquals(ConnectionUse.SAVED_TRUST, access.current())
        assertThrows(IllegalStateException::class.java) { access.acquire(ConnectionUse.CONNECTION) }
        native.failRetire = false; native.release.complete(Unit)
        run.retire()
        assertNull(access.current())
    }

    @Test fun t1167UnexecutedCancelledAndRejectedRunsNeverClaimAnotherOwner() = runBlocking<Unit> {
        val access = ConnectionAccess()
        val native = Run().apply { release.complete(Unit) }
        val unstarted = access.guard(ConnectionUse.CONNECTION, native)
        unstarted.retire()
        assertTrue(runCatching { unstarted.execute() }.isFailure)
        assertEquals(0, native.executions); assertNull(access.current())
        val cancelled = access.guard(ConnectionUse.CONNECTION, Run().apply { release.complete(Unit) })
        cancelled.cancel(); cancelled.retire()
        assertTrue(runCatching { cancelled.execute() }.isFailure)
        access.acquire(ConnectionUse.SAVED_TRUST).use {
            val rejected = access.guard(ConnectionUse.CONNECTION, Run().apply { release.complete(Unit) })
            assertTrue(runCatching { rejected.execute() }.isFailure)
            rejected.retire()
            assertEquals(ConnectionUse.SAVED_TRUST, access.current())
        }
    }
}
