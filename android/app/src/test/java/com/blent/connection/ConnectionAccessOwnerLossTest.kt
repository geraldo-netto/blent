package com.blent.connection

import com.blent.LifecycleRun
import com.blent.ResourceLane
import kotlinx.coroutines.*
import org.junit.Assert.*
import org.junit.Test

class ConnectionAccessOwnerLossTest {
    private class Held : LifecycleRun {
        val entered = CompletableDeferred<Unit>()
        val retiring = CompletableDeferred<Unit>()
        val release = CompletableDeferred<Unit>()
        override suspend fun execute() { entered.complete(Unit); awaitCancellation() }
        override fun cancel() {}
        override suspend fun retire() { retiring.complete(Unit); release.await() }
    }

    @Test fun t1167OwnerScopeLossKeepsActualNativeRetirementFunded() = runBlocking<Unit> {
        val access = ConnectionAccess()
        val native = Held()
        val scope = CoroutineScope(SupervisorJob() + Dispatchers.Unconfined)
        val lane = ResourceLane(scope, {})
        lane.replace({ access.guard(ConnectionUse.CONNECTION, native) }, {})
        try {
            native.entered.await()
            scope.cancel()
            native.retiring.await()
            assertEquals(ConnectionUse.CONNECTION, access.current())
            assertThrows(IllegalStateException::class.java) { access.acquire(ConnectionUse.SAVED_TRUST) }
        } finally { native.release.complete(Unit); scope.coroutineContext[Job]!!.join() }
        assertNull(access.current())
    }

    @Test fun t1167ExecutionCancellationDoesNotReleaseNativeOwnership() = runBlocking<Unit> {
        val access = ConnectionAccess(); val native = Held()
        val run = access.guard(ConnectionUse.SAVED_TRUST, native)
        val execution = launch(start = CoroutineStart.UNDISPATCHED) { run.execute() }
        try {
            native.entered.await(); execution.cancelAndJoin()
            assertEquals(ConnectionUse.SAVED_TRUST, access.current())
            val cleanup = async(start = CoroutineStart.UNDISPATCHED) { run.retire() }
            assertFalse(cleanup.isCompleted)
            native.release.complete(Unit); cleanup.await()
        } finally { native.release.complete(Unit); execution.cancelAndJoin(); run.retire() }
        assertNull(access.current())
    }
}
