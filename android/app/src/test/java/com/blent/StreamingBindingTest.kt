package com.blent

import android.os.Looper
import kotlinx.coroutines.*
import kotlinx.coroutines.flow.MutableStateFlow
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.Shadows.shadowOf
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [27, 34])
class StreamingBindingTest {
    private fun idle() = shadowOf(Looper.getMainLooper()).idle()
    private fun owner(rig: PowerRuntimeFixture) = checkNotNull(PowerServiceNotice.read(rig.app.starts.last())).notice.original.owner

    @Test fun t1069EqualPowerValuesStillPublishAChangedOriginalSourceView() = runBlocking {
        val source = com.blent.activity.DisplayActivitySource()
        val before = source.update
        val power = StreamingPower()
        val updates = MutableStateFlow(DisplayPowerDemand(power, DisplayPowerPermit { source.current(before) }))
        val rig = PowerRuntimeFixture()
        val binding = StreamingPowerBinding(rig.app, rig.runtime)
        binding.start(updates.value, updates); idle()
        val original = owner(rig)
        try {
            source.replace()
            val current = source.update
            updates.value = DisplayPowerDemand(power, DisplayPowerPermit { source.current(current) }); idle()
            assertEquals(1, rig.app.starts.size)
            rig.attachFirst()
            assertEquals("T1069 equal policy suppressed a fresh original permit", 1, rig.native.size)
        } finally { binding.stop(); rig.finish(original) }
    }
    @Test fun t497_completedPolicyFlowKeepsTheServiceOwnedUntilStop() = runBlocking {
        val rig = PowerRuntimeFixture()
        val binding = StreamingPowerBinding(rig.app, rig.runtime)
        val initial = DisplayPowerDemand(StreamingPower(false))
        binding.start(initial, kotlinx.coroutines.flow.flowOf(initial, DisplayPowerDemand(StreamingPower(true))))
        idle()
        val original = owner(rig)
        try {
            assertEquals("T1060 current policy must coalesce behind original native admission", 1, rig.app.starts.size)
            assertTrue(rig.runtime.owns(original))
            rig.attachFirst()
            assertTrue("T497 stale initial normal policy was applied instead of latest battery policy", rig.native.isEmpty())
            assertEquals(0, rig.stops)
            binding.stop()
            assertFalse(rig.runtime.owns(original)); assertEquals(1, rig.stops)
        } finally { binding.stop(); rig.finish(original) }
    }
    @Test fun t388_stop_cancels_updates_and_restart_takes_current_policy() = runBlocking {
        val rig = PowerRuntimeFixture()
        val binding = StreamingPowerBinding(rig.app, rig.runtime)
        val updates = MutableStateFlow(DisplayPowerDemand(StreamingPower(true, true, StreamTransport.USB)))
        binding.start(updates.value, updates); binding.start(updates.value, updates); idle()
        assertEquals(1, rig.app.starts.size)
        assertTrue(rig.app.starts.all { it.getBooleanExtra("battery_saver", false) })
        rig.attachFirst()
        binding.stop(); rig.drain(); rig.runtime.detach(rig.port)
        val count = rig.app.starts.size + rig.notices.size
        updates.value = DisplayPowerDemand(StreamingPower(false, true, StreamTransport.NETWORK)); idle()
        assertEquals("T388 retired Activity restarted the service", count, rig.app.starts.size + rig.notices.size)
        binding.start(updates.value, updates); idle()
        val fresh = owner(rig)
        try {
            assertEquals(2, rig.app.starts.size)
            assertFalse(rig.app.starts.last().getBooleanExtra("battery_saver", true))
            rig.attachFirst()
            assertEquals(listOf(MediaTask.Wake(true, true)), rig.native)
            binding.stop()
            val stopped = rig.stops
            binding.stop(); assertEquals(stopped, rig.stops)
            assertFalse(rig.runtime.owns(fresh))
        } finally { binding.stop(); rig.finish(fresh) }
    }
    @Test fun t388_rejected_start_and_update_do_not_crash_or_keep_observing() = runBlocking {
        val rig = PowerRuntimeFixture()
        val binding = StreamingPowerBinding(rig.app, rig.runtime)
        val updates = MutableStateFlow(DisplayPowerDemand(StreamingPower(true)))
        rig.app.denied = true
        binding.start(updates.value, updates); idle()
        assertTrue(rig.app.starts.isEmpty())
        updates.value = DisplayPowerDemand(StreamingPower(true, true)); idle()
        assertTrue(rig.app.starts.isEmpty())
        rig.app.denied = false
        binding.start(updates.value, updates); idle()
        val original = owner(rig)
        try {
            rig.attachFirst(); assertEquals(1, rig.native.size)
            rig.postRefused = true
            updates.value = DisplayPowerDemand(StreamingPower(true, true, StreamTransport.USB)); idle()
            assertFalse(rig.runtime.owns(original))
            assertEquals("T388 native original task was not retired exactly once", 1, rig.settled)
            val count = rig.app.starts.size + rig.notices.size
            updates.value = DisplayPowerDemand(StreamingPower(false)); idle()
            assertEquals(count, rig.app.starts.size + rig.notices.size)
            binding.stop()
        } finally { binding.stop(); rig.finish(original) }
    }
}
