package com.blent.connection

import kotlinx.coroutines.*
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [27, 34])
class SavedTrustProcessTest {
    @Test fun t1151SilentMaintenanceHasLogicalDeadlineButRetainsOriginalLiveBinder() = runBlocking<Unit> {
        val endpoint = ConnectionWorkerEndpoint().apply { start = {}; killOnExit = false }
        val context = ConnectionWorkerContext(endpoint)
        val worker = AndroidConnectionWorkerRun(context, ConnectionProcessLaunch.Trust(SavedTrustRequest.List),
            startMillis = 1000, graceMillis = 1, retireMillis = 250)
        val execution = async(Dispatchers.Default) { runCatching { worker.execute() } }
        val detected: Boolean
        var retained = false
        try {
            withTimeout(5000) { while (!endpoint.codes.contains(ConnectionWorkerIpc.START_TRUST)) delay(1) }
            val result = withTimeoutOrNull(2000) { execution.await() }
            detected = result?.exceptionOrNull() is TimeoutCancellationException
            worker.cancel()
            try { worker.retire() } catch (_: Exception) { retained = endpoint.alive && context.unbinds == 0 }
        } finally {
            endpoint.die() // Independent original-Binder death, never a timeout proof.
            execution.cancelAndJoin()
            worker.retire()
        }
        assertTrue("T1151 silent native maintenance had no progress deadline", detected)
        assertTrue("T1151 logical timeout discarded original native ownership", retained)
        assertEquals(1, context.unbinds)
    }
}
