package com.blent

import android.Manifest
import android.media.*
import kotlinx.coroutines.runBlocking
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.*
import org.robolectric.annotation.Config
import org.robolectric.annotation.Implementation
import org.robolectric.annotation.Implements
import org.robolectric.shadows.*
import org.robolectric.util.ReflectionHelpers

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [27, 34], shadows = [AudioSpeakerTest.RoutedTrack::class])
class AudioSpeakerTest {
    // Robolectric omits the JNI route-selection return value. Real routing is
    // separately checked on the tablet; this fixture exercises our adapter.
    @Implements(AudioTrack::class)
    class RoutedTrack : ShadowAudioTrack() {
        companion object { var shortWrites = 0; var reply: Int? = null; var received: ShortArray? = null }
        @Implementation protected fun native_write_short(data: ShortArray, offset: Int, count: Int, format: Int, blocking: Boolean): Int {
            assertFalse("T797 typed write became blocking", blocking)
            assertEquals(AudioFormat.ENCODING_PCM_16BIT, format)
            received = data.copyOfRange(offset, offset + count)
            shortWrites++
            reply?.let { return it }
            // Robolectric has only the byte JNI implementation. Adapt sample
            // writes into that fixture so listeners/head counters still run.
            val bytes = java.nio.ByteBuffer.allocate(count * 2).order(java.nio.ByteOrder.LITTLE_ENDIAN)
            for (index in offset until offset + count) bytes.putShort(data[index])
            val written = super.native_write_byte(bytes.array(), 0, bytes.capacity(), format, blocking)
            return if (written >= 0) written / 2 else written
        }
        @Implementation protected fun native_setOutputDevice(id: Int) = true
    }
    private fun attributes() = AudioAttributes.Builder().setUsage(AudioAttributes.USAGE_MEDIA).build()
    @Test fun t797_nativeTypedWritesKeepSampleUnitsAndDeadCounts() {
        val app = RuntimeEnvironment.getApplication()
        AndroidSpeakerTrack(app, attributes(), AudioPreferences(builtIn = false)).use { native ->
            try {
                val samples = shortArrayOf(Short.MIN_VALUE, -1, 0, Short.MAX_VALUE, 99, -99)
                RoutedTrack.reply = 2
                assertEquals(OutputWrite(2), native.write(samples, 2, 4))
                assertArrayEquals(samples.copyOfRange(2, 6), RoutedTrack.received)
                RoutedTrack.reply = 0
                assertEquals(OutputWrite(0), native.write(samples, 0, 2))
                RoutedTrack.reply = AudioTrack.ERROR_DEAD_OBJECT
                assertEquals(OutputWrite(0, true), native.write(samples, 0, 2))
                RoutedTrack.reply = AudioTrack.ERROR_INVALID_OPERATION
                assertThrows(IllegalStateException::class.java) { native.write(samples, 0, 2) }
            } finally { RoutedTrack.reply = null }
        }
    }

    @Test fun t719_focusDenialTransientDuckPermanentLossAndLateCallbacks() {
        val manager = RuntimeEnvironment.getApplication().getSystemService(AudioManager::class.java)
        val shadow = Shadows.shadowOf(manager)
        shadow.setNextFocusRequestResponse(AudioManager.AUDIOFOCUS_REQUEST_FAILED)
        try { AudioFocus(manager, attributes()); fail("denied focus played") } catch (_: IllegalStateException) {}
        assertNotNull(shadow.lastAbandonedAudioFocusRequest)
        shadow.setNextFocusRequestResponse(AudioManager.AUDIOFOCUS_REQUEST_GRANTED)
        val focus = AudioFocus(manager, attributes())
        val request = shadow.lastAudioFocusRequest.audioFocusRequest
        val listener = shadow.lastAudioFocusRequest.listener!!
        assertFalse(focus.paused())
        for (loss in listOf(AudioManager.AUDIOFOCUS_LOSS_TRANSIENT, AudioManager.AUDIOFOCUS_LOSS_TRANSIENT_CAN_DUCK)) {
            listener.onAudioFocusChange(loss); assertTrue(focus.paused())
            listener.onAudioFocusChange(AudioManager.AUDIOFOCUS_GAIN); assertFalse(focus.paused())
        }
        listener.onAudioFocusChange(AudioManager.AUDIOFOCUS_LOSS)
        listener.onAudioFocusChange(AudioManager.AUDIOFOCUS_GAIN)
        try { focus.paused(); fail("permanent focus loss resumed") } catch (_: IllegalStateException) {}
        focus.close(); focus.close(); assertSame(request, shadow.lastAbandonedAudioFocusRequest)
        listener.onAudioFocusChange(AudioManager.AUDIOFOCUS_GAIN)
        try { focus.paused(); fail("closed focus resumed") } catch (_: IllegalStateException) {}
    }
    @Test fun t719_nativeStereoPlaybackNeedsNoRecordPermissionAndReleases() = runBlocking {
        RoutedTrack.shortWrites = 0
        val app = RuntimeEnvironment.getApplication()
        Shadows.shadowOf(app).denyPermissions(Manifest.permission.RECORD_AUDIO)
        val endpoint = AudioEndpoint("a".repeat(64), 12345, 2, 1, 40, false)
        for (processing in 1..2) AudioSpeaker.open(app, endpoint.copy(processing = processing), AudioPreferences(builtIn = false)).use { player ->
            player.start(); assertFalse(player.paused)
            assertTrue(player.description().contains("native buffer"))
            assertEquals(SpeakerWrite.Written, player.write(PlaybackChunk(ShortArray(960), android.os.SystemClock.elapsedRealtime(), false)))
        }
        assertTrue("T797 speaker serialized PCM instead of typed write", RoutedTrack.shortWrites > 0)
        try { AudioSpeaker.open(app, endpoint.copy(direction = 1), AudioPreferences()); fail("microphone sent to output") } catch (_: IllegalArgumentException) {}
        try { AudioSpeaker.open(app, endpoint, AudioPreferences(gain = 101)); fail("invalid volume") } catch (_: IllegalArgumentException) {}
    }
    @Test fun t719_nativeRouteAndBufferBounds() {
        val app = RuntimeEnvironment.getApplication()
        val manager = app.getSystemService(AudioManager::class.java)
        val device = AudioDeviceInfoBuilder.newBuilder().setType(AudioDeviceInfo.TYPE_BUILTIN_SPEAKER).build()
        val port = ReflectionHelpers.getField<Any>(device, "mPort")
        ReflectionHelpers.setField(port, "mName", "fixture speaker")
        ReflectionHelpers.setField(port, "mRole", 2); ReflectionHelpers.setField(port, "mType", 2)
        Shadows.shadowOf(manager).setOutputDevices(listOf(device))
        AndroidSpeakerTrack(app, attributes(), AudioPreferences()).use { native ->
            assertNull(native.route()); native.play(); native.flush()
            ShadowAudioTrack.setRoutedDevice(device)
            assertEquals(device.id, native.route()); assertTrue(native.description().contains("native buffer"))
            assertEquals(2, native.write(shortArrayOf(513, 1027), 0, 2).samples)
            try { native.write(ShortArray(2), -1, 2); fail("invalid native write") } catch (_: IllegalStateException) {}
            native.close(); native.close()
        }
        ShadowAudioTrack.setRoutedDevice(null)
        Shadows.shadowOf(manager).setOutputDevices(emptyList())
        try { AndroidSpeakerTrack(app, attributes(), AudioPreferences()); fail("missing built-in speaker") } catch (_: IllegalStateException) {}
        ShadowAudioTrack.setMinBufferSize(AudioTrack.ERROR_BAD_VALUE)
        try { AndroidSpeakerTrack(app, attributes(), AudioPreferences()); fail("invalid native buffer") } catch (_: IllegalStateException) {}
    }
}
