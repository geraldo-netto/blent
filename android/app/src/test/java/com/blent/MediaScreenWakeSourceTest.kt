package com.blent

import android.os.Looper
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.flowOf
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.Shadows
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [27, 34])
class MediaScreenWakeSourceTest {
    private class Sources {
        val camera = MutableStateFlow(CameraLifecycleState())
        val microphone = MutableStateFlow(AudioLifecycleState(AudioPreferences(), "off"))
        val speakers = MutableStateFlow(AudioLifecycleState(AudioPreferences(), "off"))
        val source = MediaScreenWakeSource(camera, microphone, speakers)
    }

    private fun idle() = Shadows.shadowOf(Looper.getMainLooper()).idle()

    @Test fun t1235EveryMediaCombinationSharesOneWindowOwnerAndStopCutsOffObservation() {
        val sources = Sources()
        val writes = ArrayList<Boolean>()
        val binding = WindowPowerBinding(writes::add)
        binding.start(sources.source.current(), sources.source.updates)
        try {
            for (lens in listOf(null, CameraLens.FRONT, CameraLens.REAR)) {
                for (microphone in listOf(false, true)) {
                    for (speakers in listOf(false, true)) {
                        sources.camera.value = CameraLifecycleState(selected = lens)
                        sources.microphone.value = sources.microphone.value.copy(sharing = microphone)
                        sources.speakers.value = sources.speakers.value.copy(sharing = speakers)
                        idle()
                        assertEquals("T1235 $lens/$microphone/$speakers", lens != null || microphone || speakers, writes.last())
                    }
                }
            }
            binding.stop()
            val stopped = writes.size
            sources.camera.value = CameraLifecycleState()
            sources.microphone.value = sources.microphone.value.copy(sharing = false)
            sources.speakers.value = sources.speakers.value.copy(sharing = false)
            idle()
            assertEquals(stopped, writes.size)
            assertFalse(writes.last())
        } finally { binding.stop() }
    }

    @Test fun t1235OriginalSnapshotCannotReviveAfterStopOrEqualReplacement() {
        val sources = Sources()
        sources.camera.value = CameraLifecycleState(selected = CameraLens.REAR)
        val original = sources.source.current()
        assertTrue(original.keepScreenOn())
        sources.camera.value = CameraLifecycleState()
        assertFalse(original.keepScreenOn())
        sources.camera.value = CameraLifecycleState(selected = CameraLens.REAR)
        assertFalse("T1235 an equal replacement cannot revive the old view", original.keepScreenOn())
        assertTrue(sources.source.current().keepScreenOn())
        sources.camera.value = CameraLifecycleState()
        sources.microphone.value = sources.microphone.value.copy(sharing = true)
        val microphone = sources.source.current()
        sources.microphone.value = sources.microphone.value.copy(sharing = false)
        sources.speakers.value = sources.speakers.value.copy(sharing = true)
        assertFalse(microphone.keepScreenOn())
        val speakers = sources.source.current()
        assertTrue(speakers.keepScreenOn())
        sources.speakers.value = sources.speakers.value.copy(sharing = false)
        assertFalse(speakers.keepScreenOn())
    }

    @Test fun t1235DisplayPauseDoesNotCancelMediaAndMediaStopDoesNotCancelDisplay() {
        val sources = Sources()
        val paused = DisplayPowerDemand(StreamingPower(display = DisplayWakeState.PAUSED))
        val display = DisplayPowerDemand(StreamingPower(display = DisplayWakeState.ACTIVE))
        assertFalse(combinedScreenWake(paused, sources.source.current()).keepScreenOn())
        sources.microphone.value = sources.microphone.value.copy(sharing = true)
        assertTrue(combinedScreenWake(paused, sources.source.current()).keepScreenOn())
        sources.microphone.value = sources.microphone.value.copy(sharing = false)
        assertTrue(combinedScreenWake(display, sources.source.current()).keepScreenOn())
        val invalid = DisplayPowerDemand(display.power, DisplayPowerPermit { false })
        assertFalse(combinedScreenWake(invalid, sources.source.current()).keepScreenOn())
    }

    @Test fun t1235FaultyDemandEndsOnlyOriginalWindowObservation() {
        val writes = ArrayList<Boolean>()
        val failures = ArrayList<Exception>()
        val failure = IllegalStateException("T1235 invalid original view")
        val updates = MutableStateFlow<ScreenWakeDemand>(ScreenWakeDemand { throw failure })
        val binding = WindowPowerBinding(writes::add, failures::add)
        binding.start(updates.value, updates)
        assertEquals(listOf(false), writes)
        assertEquals(listOf(failure), failures)
        updates.value = ScreenWakeDemand { true }
        idle()
        assertEquals(listOf(false), writes)
        try {
            binding.start(updates.value, flowOf(updates.value)); idle()
            assertTrue(writes.last())
        } finally { binding.stop() }
        assertFalse(writes.last())
    }
}
