package com.blent

import com.blent.activity.DisplayActivitySource
import kotlinx.coroutines.*
import kotlinx.coroutines.flow.*
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.Shadows.shadowOf
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [27, 34])
class WindowPowerBindingTest {
    private fun idle() = shadowOf(android.os.Looper.getMainLooper()).idle()
    private fun active() = DisplayPowerDemand(StreamingPower(display = DisplayWakeState.ACTIVE))
    private fun paused() = DisplayPowerDemand(StreamingPower(display = DisplayWakeState.PAUSED))
    @Test fun t1061CompletedFlowKeepsOnlyForegroundWindowDemandUntilStop() {
        val writes = ArrayList<Boolean>()
        val binding = WindowPowerBinding(writes::add)
        try {
            binding.start(active(), flowOf(active(), paused())); idle()
            assertFalse(writes.last())
            binding.start(active(), flowOf(active())); idle()
            assertFalse("T1061 duplicate start replaced original window claim", writes.last())
            binding.stop(); assertFalse(writes.last())
            val count = writes.size
            binding.stop(); assertEquals(count, writes.size)
            binding.start(active(), flowOf(active())); idle()
            assertTrue(writes.last())
        } finally { binding.stop() }
        assertFalse(writes.last())
    }
    @Test fun t1061StaleSourceCannotReacquireFromQueuedOrRepeatedValues() {
        val source = DisplayActivitySource()
        val original = source.update
        val old = DisplayPowerDemand(StreamingPower(), ActivityPowerPermit(source, original))
        val updates = MutableStateFlow(old)
        val writes = ArrayList<Boolean>()
        val binding = WindowPowerBinding(writes::add)
        try {
            source.replace()
            binding.start(old, updates); idle()
            assertTrue(writes.none { it })
            val current = source.update
            updates.value = DisplayPowerDemand(StreamingPower(), ActivityPowerPermit(source, current)); idle()
            assertTrue(writes.last())
            binding.stop()
            updates.value = active(); idle()
            assertFalse(writes.last())
        } finally { binding.stop() }
    }
    @Test fun t1061ReentrantInitialWriteCannotInstallAnOldObserverAfterReplacement() {
        val old = MutableStateFlow(active())
        val current = MutableStateFlow(paused())
        val writes = ArrayList<Boolean>()
        var replace = true
        lateinit var binding: WindowPowerBinding
        binding = WindowPowerBinding({ keep ->
            writes.add(keep)
            if (keep && replace) { replace = false; binding.stop(); binding.start(current.value, current) }
        })
        try {
            binding.start(old.value, old); idle()
            assertFalse(writes.last())
            val count = writes.size
            old.value = paused(); idle()
            assertEquals("T1061 old initial owner installed a zombie observer", count, writes.size)
            current.value = active(); idle()
            assertTrue(writes.last())
        } finally { binding.stop() }
    }
    @Test fun t1061WriteAndReporterFaultsCutOffOnlyOriginalWindowUpdates() {
        val writes = ArrayList<Boolean>()
        val updates = MutableStateFlow(active())
        val failure = IllegalStateException("T1061 native flag write failed")
        val report = IllegalStateException("T1061 reporter failed")
        var fail = true
        val binding = WindowPowerBinding({ keep -> writes.add(keep); if (fail) throw failure }, { throw report })
        try {
            binding.start(updates.value, updates); idle()
            assertTrue(failure.suppressed.contains(report))
            val count = writes.size
            updates.value = paused(); idle()
            assertEquals(count, writes.size)
            fail = false
            binding.start(active(), flowOf(active())); idle()
            assertTrue(writes.last())
        } finally { fail = false; binding.stop() }
        assertFalse(writes.last())
    }
    @Test fun t1061FailedFlowClearsItsWindowAndCancellationIsNotAReporterFailure() = runBlocking {
        val writes = ArrayList<Boolean>()
        val reports = ArrayList<Exception>()
        val gate = CompletableDeferred<Unit>()
        val failure = IllegalStateException("T1061 window policy failed")
        val binding = WindowPowerBinding(writes::add, reports::add)
        try {
            binding.start(active(), flow { gate.await(); throw failure }); idle()
            assertTrue(writes.last())
            gate.complete(Unit); idle()
            assertFalse(writes.last()); assertEquals(listOf(failure), reports)
            binding.start(active(), MutableStateFlow(active())); idle()
            binding.stop(); idle()
            assertEquals(listOf(failure), reports)
        } finally { gate.complete(Unit); binding.stop() }
    }
    @Test fun t1061CanceledInputClearsOriginalWindowWithoutReportingNormalCancellation() = runBlocking {
        val writes = ArrayList<Boolean>()
        val reports = ArrayList<Exception>()
        val gate = CompletableDeferred<Unit>()
        val binding = WindowPowerBinding(writes::add, reports::add)
        try {
            binding.start(active(), flow { gate.await(); throw CancellationException("T1061 canceled source") }); idle()
            assertTrue(writes.last())
            gate.complete(Unit); idle()
            assertFalse("T1061 canceled policy retained its window flag", writes.last())
            assertTrue(reports.isEmpty())
        } finally { gate.complete(Unit); binding.stop() }
    }
    @Test fun t1061DefaultReporterContainsNativeWriteFailureAfterCutoff() {
        val failure = IllegalStateException("T1061 default native flag failure")
        val writes = ArrayList<Boolean>()
        val binding = WindowPowerBinding({ keep -> if (keep) throw failure else writes.add(false) })
        try {
            binding.start(active(), flowOf(active())); idle()
            assertEquals(listOf(false), writes)
            assertTrue(org.robolectric.shadows.ShadowLog.getLogsForTag("BlentWindow").any { it.throwable === failure })
        } finally { binding.stop() }
    }
    @Test fun t1061ReentrantReaderCannotWriteForItsOldWindowEpoch() {
        val writes = ArrayList<Boolean>()
        var change = true
        lateinit var binding: WindowPowerBinding
        val old = DisplayPowerDemand(StreamingPower(), DisplayPowerPermit {
            if (change) { change = false; binding.stop(); binding.start(paused(), flowOf(paused())) }
            true
        })
        binding = WindowPowerBinding(writes::add)
        try {
            binding.start(old, flowOf(old)); idle()
            assertTrue("T1061 reentrant old reader wrote true after epoch cutoff", writes.none { it })
        } finally { binding.stop() }
    }
}
