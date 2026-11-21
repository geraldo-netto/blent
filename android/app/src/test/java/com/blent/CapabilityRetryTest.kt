package com.blent

import kotlinx.coroutines.*
import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [27, 34])
class CapabilityRetryTest {
    @Test fun t1560FailedAndBusyQueriesUseBoundedBackoff() = runBlocking {
        val delays = mutableListOf<Long>()
        var calls = 0
        val ready = JSONObject().put("codecs", org.json.JSONArray())
        val result = awaitCapabilities(query = {
            calls++
            when (calls) {
                1 -> throw IllegalStateException("worker unavailable")
                in 2..8 -> JSONObject().put("status", "unavailable")
                else -> ready
            }
        }, wait = { delays.add(it) })
        assertSame(ready, result)
        assertEquals(listOf(100L, 200L, 400L, 800L, 1600L, 2000L, 2000L, 2000L), delays)
        assertEquals(9, calls)
    }

    @Test fun t1560CancellationDoesNotBecomeAnotherQuery() = runBlocking {
        var calls = 0
        val job = launch(start = CoroutineStart.UNDISPATCHED) {
            awaitCapabilities(query = { calls++; throw CancellationException("original cancelled") },
                wait = { error("cancellation must propagate") })
        }
        job.join()
        assertTrue(job.isCancelled)
        assertEquals(1, calls)
    }

    @Test fun t1560CancellationDuringBackoffPreventsFurtherAdmission() = runBlocking {
        val waiting = CompletableDeferred<Unit>()
        var calls = 0
        val job = launch {
            awaitCapabilities(query = { calls++; JSONObject().put("status", "unavailable") },
                wait = { waiting.complete(Unit); awaitCancellation() })
        }
        waiting.await()
        job.cancelAndJoin()
        assertEquals(1, calls)
    }
}
