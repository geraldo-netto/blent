package com.blent.connection

import kotlinx.coroutines.*
import org.junit.Assert.*
import org.junit.Test

class ConnectionAccessBindingTest {
    private class Worker : ConnectionWorker {
        val release = CompletableDeferred<Unit>()
        var executions = 0
        override suspend fun execute() { executions++; awaitCancellation() }
        override fun command(value: ConnectionWorkerCommand) = false
        override fun cancel() {}
        override suspend fun retire() { release.await() }
    }

    @Test fun t1167ConnectionViewEndingDoesNotReleaseSharedNativeAccess() = runBlocking<Unit> {
        val access = ConnectionAccess(); val worker = Worker()
        val scope = CoroutineScope(SupervisorJob() + Dispatchers.Unconfined)
        val binding = ConnectionBinding(scope, { _, _ -> worker }, access = access)
        try {
            binding.connect(ConnectionWorkerSpec("fixture"))
            assertEquals(1, worker.executions); assertEquals(ConnectionUse.CONNECTION, access.current())
            binding.disconnect()
            assertFalse(binding.state.value.active)
            assertNotNull(binding.state.value.cleanup)
            assertEquals(ConnectionUse.CONNECTION, access.current())
            assertThrows(IllegalStateException::class.java) { access.acquire(ConnectionUse.SAVED_TRUST) }
        } finally { worker.release.complete(Unit); scope.cancel(); scope.coroutineContext[Job]!!.join() }
        assertNull(access.current()); assertNull(binding.state.value.cleanup)
    }

    @Test fun t1167MaintenanceOwnershipPreventsConnectionExecutionAndSurvivesItsCleanup() = runBlocking<Unit> {
        val access = ConnectionAccess(); val worker = Worker().apply { release.complete(Unit) }
        val scope = CoroutineScope(SupervisorJob() + Dispatchers.Unconfined)
        val binding = ConnectionBinding(scope, { _, _ -> worker }, access = access)
        try {
            access.acquire(ConnectionUse.SAVED_TRUST).use {
                binding.connect(ConnectionWorkerSpec("fixture"))
                assertEquals(0, worker.executions)
                assertFalse(binding.state.value.active); assertNull(binding.state.value.cleanup)
                assertEquals(ConnectionUse.SAVED_TRUST, access.current())
            }
        } finally { scope.cancel(); scope.coroutineContext[Job]!!.join() }
        assertNull(access.current())
    }
}
