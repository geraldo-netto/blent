package com.blent.connection

import android.content.Intent
import android.hardware.usb.UsbManager
import com.blent.*
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.Robolectric
import org.robolectric.RuntimeEnvironment
import org.robolectric.annotation.Config

@RunWith(com.blent.ComposeFixtureRunner::class)
@Config(sdk = [27, 34], instrumentedPackages = ["androidx.compose.ui.platform"])
class ConnectionReattachTest {
    @Test fun t1392AttachmentDuringOldExecutionSurvivesUntilRetirement() {
        ConnectionUiFixture().use { fixture ->
            WorkerTestBridge.install()
            Prefs(RuntimeEnvironment.getApplication()).checkUpdates = false
            val activity = Robolectric.buildActivity(MainActivity::class.java,
                Intent(UsbManager.ACTION_USB_ACCESSORY_ATTACHED)).create()
            try {
                val original = fixture.workers.single()
                // T1424: replacement requires a real observed attachment epoch.
                fixture.nativeAttachment.reattach()
                activity.newIntent(Intent(UsbManager.ACTION_USB_ACCESSORY_ATTACHED))
                activity.newIntent(Intent(UsbManager.ACTION_USB_ACCESSORY_ATTACHED))
                assertEquals(1, fixture.workers.size)
                assertEquals(0, original.cancelled)
                original.released.complete(Unit)
                assertEquals("T1392: attachment was discarded while the old worker settled", 2, fixture.workers.size)
                assertTrue(fixture.workers.last().commands.isEmpty())
            } finally { activity.destroy(); CameraOwner.reset(); AudioOwner.reset() }
        }
    }

    @Test fun t1424DuplicateAttachmentCannotMintAReplacementEpoch() {
        ConnectionUiFixture().use { fixture ->
            WorkerTestBridge.install()
            Prefs(RuntimeEnvironment.getApplication()).checkUpdates = false
            val activity = Robolectric.buildActivity(MainActivity::class.java,
                Intent(UsbManager.ACTION_USB_ACCESSORY_ATTACHED)).create()
            try {
                val original = fixture.workers.single()
                activity.newIntent(Intent(UsbManager.ACTION_USB_ACCESSORY_ATTACHED))
                activity.newIntent(Intent(UsbManager.ACTION_USB_ACCESSORY_ATTACHED))
                assertEquals(1, fixture.workers.size)
                assertEquals(0, original.cancelled)
                original.released.complete(Unit)
                org.robolectric.Shadows.shadowOf(android.os.Looper.getMainLooper()).idle()
                assertEquals("T1424 duplicate Intent is not a new native attachment", 1, fixture.workers.size)
            } finally { activity.destroy(); CameraOwner.reset(); AudioOwner.reset() }
        }
    }
}
