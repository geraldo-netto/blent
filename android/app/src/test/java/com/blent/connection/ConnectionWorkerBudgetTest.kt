package com.blent.connection

import kotlinx.coroutines.*
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config
import java.io.IOException
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [27, 34])
class ConnectionWorkerBudgetTest {
    private val spec = ConnectionWorkerSpec("selected-accessory")
    private suspend fun until(condition: () -> Boolean) = withTimeout(1500) { while (!condition()) delay(1) }

    @Test fun t1519BindingExpiryIsFailureAndRetiresOnlyUnexposedBinding() = runBlocking {
        val endpoint = ConnectionWorkerEndpoint()
        val context = ConnectionWorkerContext(endpoint).apply { connect = false }
        val worker = AndroidConnectionWorkerRun(context, spec, bindMillis = 20, startMillis = 1000, graceMillis = 1)
        try {
            val error = runCatching { worker.execute() }.exceptionOrNull()
            assertTrue("T1519 binding timeout was cancellation: $error", error is IOException)
            assertTrue(checkNotNull(error).message!!.contains("binding"))
            assertTrue(endpoint.codes.isEmpty())
        } finally { worker.retire() }
        assertEquals(1, context.unbinds)
    }

    @Test fun t1519StartExpiryIsFailureWhileBlockedOriginalIpcRemainsOwned() = runBlocking {
        val entered = CountDownLatch(1)
        val release = CountDownLatch(1)
        val endpoint = ConnectionWorkerEndpoint().apply { start = {
            entered.countDown(); check(release.await(3, TimeUnit.SECONDS))
        } }
        val context = ConnectionWorkerContext(endpoint)
        val worker = AndroidConnectionWorkerRun(context, spec, startMillis = 100, graceMillis = 1, retireMillis = 30)
        val execution = async(Dispatchers.Default) { runCatching { worker.execute() } }
        try {
            withContext(Dispatchers.IO) { check(entered.await(3, TimeUnit.SECONDS)) }
            val error = execution.await().exceptionOrNull()
            assertTrue("T1519 start timeout was cancellation: $error", error is IOException)
            assertTrue(runCatching { worker.retire() }.isFailure)
            assertEquals(0, context.unbinds)
        } finally { release.countDown(); endpoint.die(); worker.retire() }
        assertEquals(1, context.unbinds)
    }

    @Test fun t1519TrustObservationDoesNotReuseTheStartupBudget() = runBlocking {
        val endpoint = ConnectionWorkerEndpoint().apply { start = {} }
        val worker = AndroidConnectionWorkerRun(ConnectionWorkerContext(endpoint),
            ConnectionProcessLaunch.Trust(SavedTrustRequest.List), startMillis = 100, graceMillis = 1)
        val execution = async(Dispatchers.Default) { runCatching { worker.execute() } }
        try {
            until { endpoint.callback != null }
            delay(150)
            assertTrue("T1519 trust operation inherited the startup deadline", execution.isActive)
            endpoint.emit(ConnectionWorkerEvent.Ended(null))
            assertTrue(execution.await().isSuccess)
        } finally { worker.cancel(); endpoint.die(); worker.retire(); execution.await() }
    }
    @Test fun t1519TrustHasItsOwnExplicitFailureBudget() = runBlocking {
        val endpoint = ConnectionWorkerEndpoint().apply { start = {} }
        val worker = AndroidConnectionWorkerRun(ConnectionWorkerContext(endpoint),
            ConnectionProcessLaunch.Trust(SavedTrustRequest.List), startMillis = 1000, trustMillis = 20, graceMillis = 1)
        try {
            val error = runCatching { worker.execute() }.exceptionOrNull()
            assertTrue(error is IOException)
            assertTrue(checkNotNull(error).message!!.contains("saved-trust operation"))
        } finally { endpoint.die(); worker.retire() }
    }

    @Test fun t1519BudgetsPreserveCallerCancellationAndNestedDeadlines() = runBlocking {
        assertEquals("ready", connectionWorkerBudget("fixture", 1000) { "ready" })
        val cancelled = runCatching { withTimeout(20) {
            connectionWorkerBudget("fixture", 1000) { delay(1000); Unit }
        } }.exceptionOrNull()
        assertTrue(cancelled is TimeoutCancellationException)
        val nested = runCatching { connectionWorkerBudget("fixture", 1000) {
            withTimeout(20) { delay(1000); Unit }
        } }.exceptionOrNull()
        assertTrue(nested is TimeoutCancellationException)
    }

    @Test fun t1519BudgetBoundsRejectBeforeRunningWork() = runBlocking {
        for (millis in listOf(Long.MIN_VALUE, -1L, 0L, 60001L, Long.MAX_VALUE)) {
            assertTrue(runCatching { connectionWorkerBudget("fixture", millis) {
                error("invalid budget admitted work")
            } }.exceptionOrNull() is IllegalArgumentException)
            assertThrows(IllegalArgumentException::class.java) {
                AndroidConnectionWorkerRun(ConnectionWorkerContext(ConnectionWorkerEndpoint()),
                    ConnectionProcessLaunch.Trust(SavedTrustRequest.List), trustMillis = millis)
            }
        }
        for (millis in listOf(1L, 60000L)) {
            assertEquals(Unit, connectionWorkerBudget("fixture", millis) { Unit })
        }
    }

}
