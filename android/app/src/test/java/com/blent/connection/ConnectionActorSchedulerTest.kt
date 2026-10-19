package com.blent.connection

import kotlin.coroutines.EmptyCoroutineContext
import kotlinx.coroutines.*
import org.junit.Assert.*
import org.junit.Test

@OptIn(InternalCoroutinesApi::class)
class ConnectionActorSchedulerTest {
    @Test fun t1096DisposedQueuedCallbackCannotReachRetiredViewOrReplacement() {
        val scheduler = ConnectionActorScheduler()
        var retired = false
        var replacementCalls = 0
        val first = scheduler.invokeOnTimeout(10, Runnable { assertFalse("T1096 retired callback ran", retired) }, EmptyCoroutineContext)
        try {
            scheduler.queueUntil(10); first.dispose(); retired = true
            val replacement = scheduler.invokeOnTimeout(0, Runnable { replacementCalls++ }, EmptyCoroutineContext)
            try {
                scheduler.advanceTo(10); scheduler.runCurrent()
                assertEquals(1, replacementCalls)
            } finally { replacement.dispose() }
        } finally { first.dispose(); scheduler.runCurrent() }
        assertTrue(scheduler.empty())
    }

    @Test fun t1096All64CancellationHistoriesKeepThreeOriginalTimersIndependent() {
        for (mask in 0..63) {
            val scheduler = ConnectionActorScheduler()
            val calls = IntArray(3)
            val handles = List(3) { index -> scheduler.invokeOnTimeout(10, Runnable { calls[index]++ }, EmptyCoroutineContext) }
            try {
                for (index in 0..2) if (mask and (1 shl index) != 0) handles[index].dispose()
                scheduler.queueUntil(10)
                for (index in 0..2) if (mask and (1 shl (index + 3)) != 0) handles[index].dispose()
                scheduler.runCurrent(); scheduler.runCurrent()
                for (index in 0..2) {
                    val cancelled = mask and (1 shl index) != 0 || mask and (1 shl (index + 3)) != 0
                    assertEquals(if (cancelled) 0 else 1, calls[index])
                }
            } finally { handles.forEach { it.dispose() }; scheduler.runCurrent() }
            assertTrue(scheduler.empty())
        }
    }

    @Test fun t1096InvalidTimesAndBothIndependentCapacitiesFailBeforeAdmission() {
        val scheduler = ConnectionActorScheduler()
        for (time in listOf(Long.MIN_VALUE, -1L)) {
            assertThrows(IllegalArgumentException::class.java) { scheduler.invokeOnTimeout(time, Runnable {}, EmptyCoroutineContext) }
            assertThrows(IllegalArgumentException::class.java) { scheduler.queueUntil(time) }
        }
        val handles = List(64) { scheduler.invokeOnTimeout(1, Runnable {}, EmptyCoroutineContext) }
        try {
            assertThrows(IllegalStateException::class.java) { scheduler.invokeOnTimeout(1, Runnable {}, EmptyCoroutineContext) }
        } finally { handles.forEach { it.dispose() } }
        repeat(64) { scheduler.dispatch(EmptyCoroutineContext, Runnable {}) }
        assertThrows(IllegalStateException::class.java) { scheduler.dispatch(EmptyCoroutineContext, Runnable {}) }
        scheduler.runCurrent(); scheduler.queueUntil(Long.MAX_VALUE)
        assertThrows(IllegalArgumentException::class.java) { scheduler.invokeOnTimeout(1, Runnable {}, EmptyCoroutineContext) }
        assertTrue(scheduler.empty())
    }

    @Test fun t1096CancelledSuspendedCoroutineSettlesOriginalDelayRegistration() {
        val scheduler = ConnectionActorScheduler()
        val scope = CoroutineScope(SupervisorJob() + scheduler)
        var resumed = false
        val work = scope.launch { delay(10); resumed = true }
        try {
            scheduler.runCurrent(); assertFalse(work.isCompleted)
            work.cancel(); scheduler.runCurrent(); assertTrue(work.isCompleted)
            scheduler.advanceTo(10); assertFalse(resumed)
        } finally { scope.cancel(); scheduler.runCurrent() }
        assertTrue(scheduler.empty())
    }
}
