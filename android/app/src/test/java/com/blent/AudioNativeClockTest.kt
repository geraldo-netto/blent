package com.blent

import android.Manifest
import android.media.*
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.*
import org.robolectric.annotation.*
import org.robolectric.shadows.ShadowAudioRecord
import org.robolectric.shadows.ShadowAudioTrack

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [27, 34], shadows = [AudioNativeClockTest.RecordClock::class, AudioNativeClockTest.TrackClock::class])
class AudioNativeClockTest {
    companion object {
        internal var sample: AudioClockSample? = null
        var timebase = -1
        val recordStamps = java.util.Collections.newSetFromMap(java.util.IdentityHashMap<AudioTimestamp, Boolean>())
        val trackStamps = java.util.Collections.newSetFromMap(java.util.IdentityHashMap<AudioTimestamp, Boolean>())
    }
    @Implements(AudioRecord::class)
    class RecordClock {
        companion object {
            @JvmStatic @Implementation protected fun native_get_min_buff_size(rate: Int, channels: Int, format: Int) = 3840
        }
        @Implementation protected fun getTimestamp(stamp: AudioTimestamp, base: Int): Int {
            recordStamps.add(stamp)
            timebase = base
            val value = sample ?: return AudioRecord.ERROR_INVALID_OPERATION
            stamp.framePosition = value.frames; stamp.nanoTime = value.nanos
            return AudioRecord.SUCCESS
        }
    }
    @Implements(AudioTrack::class)
    class TrackClock : ShadowAudioTrack() {
        @Implementation protected fun getTimestamp(stamp: AudioTimestamp): Boolean {
            trackStamps.add(stamp)
            val value = sample ?: return false
            stamp.framePosition = value.frames; stamp.nanoTime = value.nanos
            return true
        }
    }
    @Test fun t720_androidAdaptersUseNativeTimestampsAndExposeUnavailableCounters() {
        recordStamps.clear(); trackStamps.clear()
        val app = RuntimeEnvironment.getApplication()
        Shadows.shadowOf(app).grantPermissions(Manifest.permission.RECORD_AUDIO)
        val endpoint = AudioEndpoint("a".repeat(64), 12345, 1, 1, 40, false)
        AudioMicrophone(app, endpoint, AudioPreferences(builtIn = false)).use { microphone ->
            AndroidSpeakerTrack(app, AudioAttributes.Builder().setUsage(AudioAttributes.USAGE_MEDIA).build(),
                AudioPreferences(builtIn = false)).use { speaker ->
                for (value in listOf(null, AudioClockSample(1, -1, 1), AudioClockSample(1, 1, 0), AudioClockSample(1, 96000, 2_000_000_001), null)) {
                    sample = value
                    val expected = value?.takeIf { it.valid() }
                    assertEquals(expected, microphone.clockSample()); assertEquals(expected, speaker.clockSample())
                    assertEquals(AudioTimestamp.TIMEBASE_MONOTONIC, timebase)
                }
            }
        }
        assertEquals("T797 microphone allocates timestamp scratch each read", 1, recordStamps.size)
        assertEquals("T797 speaker allocates timestamp scratch each read", 1, trackStamps.size)
        sample = null
    }
}
