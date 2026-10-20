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
class ConnectionAccessBinderTest {
    @Test fun t1167EndedAndRetirementDeadlineCannotAdmitMaintenanceBeforeOriginalBinderDeath() = runBlocking<Unit> {
        val access = ConnectionAccess()
        val endpoint = ConnectionWorkerEndpoint().apply { killOnExit = false }
        val context = ConnectionWorkerContext(endpoint)
        val native = AndroidConnectionWorkerRun(context, ConnectionWorkerSpec("fixture"), graceMillis = 1, retireMillis = 100)
        val original = access.guard(ConnectionUse.CONNECTION, native)
        try {
            original.execute()
            assertTrue(endpoint.alive); assertEquals(ConnectionUse.CONNECTION, access.current())
            val other = ConnectionWorkerContext(ConnectionWorkerEndpoint())
            val rejected = access.guard(ConnectionUse.SAVED_TRUST, AndroidConnectionWorkerRun(other,
                ConnectionProcessLaunch.Trust(SavedTrustRequest.List), graceMillis = 1, retireMillis = 100))
            val result = try { runCatching { rejected.execute() } } finally { rejected.retire() }
            assertTrue(result.isFailure)
            assertEquals(0, other.binds); assertEquals(0, other.unbinds)
            assertTrue(other.endpoint.codes.isEmpty())
            assertTrue(runCatching { original.retire() }.isFailure)
            assertEquals(0, context.unbinds); assertEquals(ConnectionUse.CONNECTION, access.current())
        } finally { endpoint.die(); original.retire() }
        assertEquals(1, context.unbinds); assertNull(access.current())
    }

    @Test fun t1167DeadBinderCannotReturnSlotWhileOriginalStopCallIsBlocked() = runBlocking<Unit> {
        val access = ConnectionAccess()
        val entered = CountDownLatch(1); val release = CountDownLatch(1)
        val endpoint = ConnectionWorkerEndpoint().apply { stop = { entered.countDown(); release.await() } }
        val context = ConnectionWorkerContext(endpoint)
        val original = access.guard(ConnectionUse.SAVED_TRUST, AndroidConnectionWorkerRun(context,
            ConnectionProcessLaunch.Trust(SavedTrustRequest.List), graceMillis = 1, retireMillis = 100))
        try {
            original.execute()
            assertTrue(withContext(Dispatchers.IO) { entered.await(5, TimeUnit.SECONDS) })
            withTimeout(5000) { while (endpoint.alive) delay(1) }
            assertTrue(runCatching { original.retire() }.isFailure)
            assertEquals(ConnectionUse.SAVED_TRUST, access.current()); assertEquals(0, context.unbinds)
            assertThrows(IllegalStateException::class.java) { access.acquire(ConnectionUse.CONNECTION) }
        } finally { release.countDown(); endpoint.die(); original.retire() }
        assertNull(access.current()); assertEquals(1, context.unbinds)
    }
}
