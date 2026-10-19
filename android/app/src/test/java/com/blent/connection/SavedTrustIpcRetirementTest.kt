package com.blent.connection

import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import kotlinx.coroutines.*
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [27, 34])
class SavedTrustIpcRetirementTest {
    private suspend fun until(condition: () -> Boolean) = withTimeout(5000) { while (!condition()) delay(1) }

    @Test fun t1151DefaultMetadataObserverAndBudgetsRetireOriginalWithoutUsbStart() = runBlocking<Unit> {
        val endpoint = ConnectionWorkerEndpoint().apply { start = {
            it.emit(ConnectionWorkerEvent.SavedComputers(SavedComputersPage(null, 0, 0, 0, emptyList())))
            it.emit(ConnectionWorkerEvent.Ended(null))
        } }
        val context = ConnectionWorkerContext(endpoint)
        val worker = AndroidConnectionWorkerRun(context, ConnectionProcessLaunch.Trust(SavedTrustRequest.List))
        try { worker.execute() } finally { endpoint.die(); worker.retire() }
        assertEquals(1, context.unbinds)
        assertTrue(endpoint.codes.contains(ConnectionWorkerIpc.START_TRUST))
        assertFalse(endpoint.codes.contains(ConnectionWorkerIpc.START))
    }

    @Test fun t1151BlockedMaintenanceStartAndStopRemainOwnedAfterOriginalBinderDeath() = runBlocking<Unit> {
        for (startBlocked in listOf(true, false)) checkBlockedCall(startBlocked)
    }

    private suspend fun checkBlockedCall(startBlocked: Boolean) = coroutineScope {
        val entered = CountDownLatch(1); val release = CountDownLatch(1)
        val endpoint = ConnectionWorkerEndpoint()
        val hold = { entered.countDown(); release.await(); Unit }
        if (startBlocked) endpoint.start = { hold() } else endpoint.stop = hold
        val context = ConnectionWorkerContext(endpoint)
        val worker = AndroidConnectionWorkerRun(context, ConnectionProcessLaunch.Trust(SavedTrustRequest.List),
            startMillis = 100, graceMillis = 1, retireMillis = 100)
        val execution = async(Dispatchers.Default) { runCatching { worker.execute() } }
        try {
            assertTrue(withContext(Dispatchers.IO) { entered.await(5, TimeUnit.SECONDS) })
            val result = withTimeout(5000) { execution.await() }
            assertEquals(startBlocked, result.exceptionOrNull() is TimeoutCancellationException)
            until { !endpoint.alive }
            assertTrue(runCatching { worker.retire() }.isFailure)
            assertEquals("T1151 original native IPC is still blocked", 0, context.unbinds)
        } finally {
            release.countDown(); endpoint.die()
            execution.cancelAndJoin()
            worker.retire() // Join the original unblocked call before unbinding/reuse.
        }
        assertEquals(1, context.unbinds)
    }
}
