package com.blent

import com.blent.activity.*
import java.util.concurrent.ConcurrentLinkedQueue
import java.util.concurrent.RejectedExecutionException
import java.util.concurrent.atomic.AtomicBoolean
import kotlinx.coroutines.*
import org.junit.Assert.*
import org.junit.Test

class DisplayActivityDeliveryTest {
    private fun update(revision: Long) = DisplayActivityUpdate(Any(), DisplayActivityState.Observed(DisplayActivitySignal(revision, 0, true)))
    private fun drain(queue: ConcurrentLinkedQueue<() -> Unit>) { while (true) { val action = queue.poll() ?: return; action() } }
    private suspend fun until(queue: ConcurrentLinkedQueue<() -> Unit>, predicate: () -> Boolean) {
        withTimeout(2000) { while (!predicate()) { drain(queue); delay(1) } }
        drain(queue)
    }
    @Test fun t1057ForegroundEpochsKeepOneScheduledDrainAndOnlyLatestObservation() = runBlocking {
        val queue = ConcurrentLinkedQueue<() -> Unit>()
        val consumed = ArrayList<DisplayActivityUpdate>()
        val delivery = DisplayActivityDelivery({ queue.add(it) }, { consumed.add(it) }, { throw it })
        delivery.submit(update(1)); assertTrue(queue.isEmpty())
        delivery.start()
        repeat(1000) { delivery.submit(update(it + 1L)) }
        withTimeout(2000) { while (queue.isEmpty()) delay(1) }
        assertEquals(1, queue.size)
        delivery.stop(); delivery.start()
        val replacement = update(1)
        delivery.submit(replacement)
        try {
            until(queue) { consumed.isNotEmpty() }
            assertEquals(listOf(replacement), consumed)
        } finally { delivery.stop(); drain(queue) }
    }
    @Test fun t1057RejectedUiScheduleCannotRetainOrLaterAdmitOriginalObservation() = runBlocking {
        val queue = ConcurrentLinkedQueue<() -> Unit>()
        val reports = ConcurrentLinkedQueue<Exception>()
        val consumed = ArrayList<DisplayActivityUpdate>()
        val reject = AtomicBoolean(true)
        val delivery = DisplayActivityDelivery({ action ->
            queue.add(action)
            if (reject.get()) throw RejectedExecutionException("T1057 queued then rejected")
        }, { consumed.add(it) }, { reports.add(it) })
        delivery.start(); delivery.submit(update(1))
        val reported = withTimeoutOrNull(500) { while (reports.isEmpty()) delay(1); true } == true
        delivery.stop(); drain(queue)
        assertTrue("T1057 rejected UI dispatch left original drain unaccounted", reported)
        assertTrue(consumed.isEmpty())
        reject.set(false); delivery.start()
        val replacement = update(2)
        delivery.submit(replacement)
        try {
            until(queue) { consumed.isNotEmpty() }
            assertEquals(listOf(replacement), consumed)
        } finally { delivery.stop(); drain(queue) }
    }
    @Test fun t1057AmbiguousDuplicateDispatchCannotResumeContinuationTwiceAndReportsAreIsolated() = runBlocking {
        val original = RejectedExecutionException("T1057 executed then rejected")
        val report = IllegalStateException("T1057 reporting failed")
        val consumed = ConcurrentLinkedQueue<DisplayActivityUpdate>()
        val delivery = DisplayActivityDelivery({ action -> action(); action(); throw original }, { consumed.add(it) }, { throw report })
        delivery.start(); delivery.submit(update(1))
        try {
            withTimeout(2000) { while (original.suppressed.isEmpty()) delay(1) }
            assertEquals(1, consumed.size)
            assertTrue(original.suppressed.contains(report))
        } finally { delivery.stop() }
    }

}
