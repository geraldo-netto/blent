package com.blent

import android.Manifest
import android.os.Looper
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.*
import org.robolectric.annotation.Config
import org.robolectric.annotation.LooperMode

@RunWith(com.blent.ComposeFixtureRunner::class)
@Config(sdk = [27, 34], instrumentedPackages = ["androidx.compose.ui.platform"])
@LooperMode(LooperMode.Mode.PAUSED)
class AudioActivityAttachmentTest {
    @Test fun t951OlderActivityStopCannotDetachReplacementAudioOrPermissionOwner() {
        WorkerTestBridge.install(); AudioOwner.reset(); CameraOwner.reset()
        Shadows.shadowOf(RuntimeEnvironment.getApplication()).denyPermissions(Manifest.permission.RECORD_AUDIO)
        val original = Robolectric.buildActivity(MainActivity::class.java).create().start()
        val replacement = Robolectric.buildActivity(MainActivity::class.java).create().start()
        try {
            assertSame(original.get().microphone, replacement.get().microphone)
            assertSame(original.get().speakers, replacement.get().speakers)
            original.stop()
            val offer = AudioEndpoint("ab".repeat(32), 12345, 1, 1, 40, false)
            AudioInvitations.microphone.value = offer
            Shadows.shadowOf(Looper.getMainLooper()).idle()
            assertSame("T951 older stop retired replacement invitation observation", offer, replacement.get().microphone.pending)
            assertFalse(replacement.get().microphone.sharing)
            assertNull(AudioInvitations.microphone.value)
        } finally {
            replacement.stop().destroy(); original.destroy()
            AudioOwner.reset(); CameraOwner.reset(); Shadows.shadowOf(Looper.getMainLooper()).idle()
        }
    }
}
