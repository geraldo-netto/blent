package com.blent.connection

import com.blent.LifecycleRun
import kotlinx.coroutines.*
import org.junit.Assert.*
import org.junit.Test

class SavedTrustBindingTest {
    private class Run(val report: (SavedComputersPage) -> Unit) : LifecycleRun {
        val finish = CompletableDeferred<Unit>()
        val close = CompletableDeferred<Unit>()
        var executeFailure: Exception? = null
        var closeFailure: Exception? = null
        var executeCount = 0
        var cancelCount = 0
        override suspend fun execute() {
            executeCount++; finish.await()
            executeFailure?.let { throw it }
        }
        override fun cancel() { cancelCount++ }
        override suspend fun retire() {
            if (executeCount == 0) return
            close.await(); closeFailure?.let { throw it }
        }
    }
    private class Fixture : AutoCloseable {
        val scope = CoroutineScope(SupervisorJob() + Dispatchers.Unconfined)
        val access = ConnectionAccess()
        val requests = mutableListOf<SavedTrustRequest>()
        val runs = mutableListOf<Run>()
        var factoryFailure = false
        var beforeReturn: () -> Unit = {}
        val binding = SavedTrustBinding(scope, access, SavedTrustRunFactory { request, report ->
            if (factoryFailure) error("factory rejected")
            requests.add(request)
            Run(report).also { runs.add(it); beforeReturn() }
        }, cleanupNoticeMs = 1)
        val client = binding.attach()
        fun publish(total: Int = 1) { savedPages(total).forEach(runs.last().report) }
        fun finish(total: Int = 1) { publish(total); runs.last().close.complete(Unit); runs.last().finish.complete(Unit) }
        override fun close() {
            runs.forEach { it.finish.complete(Unit); it.close.complete(Unit) }
            client.close(); scope.cancel()
        }
        suspend fun settled() { scope.coroutineContext[Job]!!.join() }
    }

    @Test fun t1168CompleteListAndConditionalForgetUseOnlyCurrentViewAndSnapshot() = runBlocking {
        val f = Fixture()
        try {
            assertTrue(f.client.refresh()); assertEquals(SavedTrustRequest.List, f.requests.single())
            f.finish()
            assertEquals(SavedTrustPhase.READY, f.binding.state.value.phase)
            val old = checkNotNull(f.client.choice(savedPeer(1)))
            assertTrue(f.client.forget(old))
            assertEquals(SavedTrustRequest.Forget(old.selected), f.requests.last())
            assertFalse(f.client.forget(old)); f.finish(0)
            assertNull(f.client.choice(savedPeer(1))); assertFalse(f.client.forget(old))
            assertNull(f.access.current())
        } finally { f.close(); f.settled() }
    }
    @Test fun t1168RecreationPreservesAcceptedWorkButRejectsEveryOldViewCommand() = runBlocking {
        val f = Fixture()
        try {
            assertTrue(f.client.refresh()); f.finish()
            val oldChoice = checkNotNull(f.client.choice(savedPeer(1)))
            assertTrue(f.client.forget(oldChoice))
            val active = f.runs.last(); val replacement = f.binding.attach()
            f.client.close()
            assertEquals(0, active.cancelCount); assertFalse(f.client.refresh()); assertFalse(f.client.stop())
            assertNull(f.client.choice(savedPeer(1))); assertFalse(replacement.forget(oldChoice))
            assertFalse(replacement.refresh()); assertEquals(ConnectionUse.SAVED_TRUST, f.access.current())
            f.finish(); assertEquals(SavedTrustPhase.READY, f.binding.state.value.phase)
            assertFalse(replacement.forget(oldChoice))
            assertNotNull(replacement.choice(savedPeer(1)))
            replacement.close(); assertNull(f.binding.state.value.viewer)
        } finally { f.close(); f.settled() }
    }
    @Test fun t1168LogicalCompletionAndDeadlineCannotPublishReadyOrReleaseNativeAccess() = runBlocking {
        val f = Fixture()
        try {
            assertTrue(f.client.refresh()); f.publish(); f.runs.single().finish.complete(Unit)
            assertEquals(SavedTrustPhase.RETIRING, f.binding.state.value.phase)
            assertNull(f.binding.state.value.computers); assertFalse(f.client.refresh())
            while (f.binding.state.value.cleanup?.contains("stalled") != true) yield()
            assertEquals(ConnectionUse.SAVED_TRUST, f.access.current())
            assertThrows(IllegalStateException::class.java) { f.access.acquire(ConnectionUse.CONNECTION) }
            f.runs.single().close.complete(Unit)
            assertEquals(SavedTrustPhase.READY, f.binding.state.value.phase); assertNull(f.access.current())
            assertTrue(f.client.refresh()); assertNull(f.binding.state.value.computers)
        } finally { f.close(); f.settled() }
    }
    @Test fun t1168CurrentViewerCanStopButCancellationNeverUndoesAnAcceptedForget() = runBlocking {
        val f = Fixture()
        try {
            assertTrue(f.client.refresh()); f.finish()
            assertTrue(f.client.forget(checkNotNull(f.client.choice(savedPeer(1)))))
            val active = f.runs.last(); val client = f.binding.attach()
            assertTrue(client.stop()); assertEquals(1, active.cancelCount)
            assertEquals(SavedTrustPhase.RETIRING, f.binding.state.value.phase)
            assertFalse(client.stop()); assertFalse(client.refresh())
            active.report(savedPages(0).single()) // callback after cutoff is discarded
            active.close.complete(Unit)
            assertEquals(SavedTrustPhase.FAILED, f.binding.state.value.phase)
            assertTrue(f.binding.state.value.failure!!.contains("may have completed"))
            assertNull(f.access.current()); client.close()
        } finally { f.close(); f.settled() }
    }
    @Test fun t1168FactoryStorageMissingAndLateResultsFailWithoutRetargeting() = runBlocking {
        val f = Fixture()
        try {
            f.factoryFailure = true; assertTrue(f.client.refresh())
            assertEquals("factory rejected", f.binding.state.value.failure); assertNull(f.access.current())
            f.factoryFailure = false; assertTrue(f.client.refresh())
            f.runs.last().apply { executeFailure = IllegalArgumentException("corrupt store"); close.complete(Unit); finish.complete(Unit) }
            assertEquals("corrupt store", f.binding.state.value.failure)
            assertTrue(f.client.refresh()); val old = f.runs.last()
            old.close.complete(Unit); old.finish.complete(Unit)
            assertTrue(f.binding.state.value.failure!!.contains("Missing"))
            assertTrue(f.client.refresh()); old.report(savedPages(1).single())
            assertEquals(SavedTrustPhase.BUSY, f.binding.state.value.phase); f.finish(0)
            assertEquals(0, f.binding.state.value.computers!!.peers.size)
        } finally { f.close(); f.settled() }
    }
    @Test fun t1168UnknownNativeCloseQuarantinesOriginalWorkAndKeepsSharedSlot() = runBlocking {
        val f = Fixture()
        try {
            assertTrue(f.client.refresh()); f.runs.single().closeFailure = IllegalStateException("close unknown")
            f.finish()
            assertEquals(SavedTrustPhase.QUARANTINED, f.binding.state.value.phase)
            assertTrue(f.binding.state.value.cleanup!!.contains("close unknown")); assertFalse(f.client.refresh())
            assertNotNull(f.access.current())
            assertThrows(IllegalStateException::class.java) { f.access.acquire(ConnectionUse.CONNECTION) }
        } finally { f.close(); f.settled() }
        // The failed lane retains the native obligation for original process loss.
        assertEquals(ConnectionUse.SAVED_TRUST, f.access.current())
    }
    @Test fun t1168ActiveConnectionAndRevokedViewerRejectBeforeFactoryAdmission() = runBlocking {
        val f = Fixture()
        try {
            f.access.acquire(ConnectionUse.CONNECTION).use {
                assertFalse(f.client.refresh()); assertTrue(f.runs.isEmpty())
                assertEquals(SavedTrustPhase.IDLE, f.binding.state.value.phase)
            }
            f.client.close(); assertFalse(f.client.refresh()); assertFalse(f.client.stop())
            assertTrue(f.runs.isEmpty())
            val current = f.binding.attach(); assertTrue(current.refresh()); f.finish(); current.close()
        } finally { f.close(); f.settled() }
    }
    @Test fun t1168OwnerScopeLossRetainsNativeCleanupUntilOriginalCloseReturns() = runBlocking {
        val f = Fixture()
        assertTrue(f.client.refresh()); val original = f.runs.single()
        try {
            f.scope.cancel()
            assertEquals(0, original.cancelCount) // Scope loss still enters non-cancellable original retirement.
            assertEquals(ConnectionUse.SAVED_TRUST, f.access.current())
        } finally { original.close.complete(Unit); f.settled(); f.client.close() }
        assertNull(f.access.current()); assertEquals(SavedTrustPhase.FAILED, f.binding.state.value.phase)
    }

    @Test fun t1168ConnectionWinningAdmissionRaceKeepsItsPermitThroughUnstartedMaintenanceCleanup() = runBlocking {
        val f = Fixture()
        var permit: ConnectionAccess.Permit? = null
        try {
            f.beforeReturn = { permit = f.access.acquire(ConnectionUse.CONNECTION) }
            assertTrue(f.client.refresh())
            assertEquals(0, f.runs.single().executeCount)
            assertEquals(SavedTrustPhase.FAILED, f.binding.state.value.phase)
            assertEquals(ConnectionUse.CONNECTION, f.access.current())
            assertTrue(f.binding.state.value.failure!!.contains("active or closing"))
        } finally { permit?.close(); f.close(); f.settled() }
        assertNull(f.access.current())
    }
}
