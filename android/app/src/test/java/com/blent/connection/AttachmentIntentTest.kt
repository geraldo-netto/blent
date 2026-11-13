package com.blent.connection

import android.content.Intent
import android.hardware.usb.UsbManager
import android.os.Looper
import com.blent.*
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.Robolectric
import org.robolectric.RuntimeEnvironment
import org.robolectric.Shadows.shadowOf
import org.robolectric.annotation.Config

@RunWith(ComposeFixtureRunner::class)
@Config(sdk = [27, 34], instrumentedPackages = ["androidx.compose.ui.platform"])
class AttachmentIntentTest {
    private enum class Cutoff { DISCONNECT, DENY, REVOKE }
    @Test fun t1405RepeatedIntentCannotUndoDisconnect() = replay(Cutoff.DISCONNECT)
    @Test fun t1405RepeatedIntentCannotUndoDenial() = replay(Cutoff.DENY)
    @Test fun t1405RepeatedIntentCannotUndoRevocation() = replay(Cutoff.REVOKE)

    private fun replay(cutoff: Cutoff) {
        ConnectionUiFixture().use { fixture ->
            WorkerTestBridge.install()
            Prefs(RuntimeEnvironment.getApplication()).checkUpdates = false
            val controller = Robolectric.buildActivity(MainActivity::class.java,
                Intent(UsbManager.ACTION_USB_ACCESSORY_ATTACHED)).create()
            try {
                assertEquals(1, fixture.workers.size)
                fixture.prompt()
                val choice = checkNotNull(fixture.binding.state.value.peer)
                when (cutoff) {
                    Cutoff.DISCONNECT -> controller.get().configuredConnection.disconnect()
                    Cutoff.DENY -> assertTrue(fixture.binding.decide(choice, false, false))
                    Cutoff.REVOKE -> assertTrue(fixture.binding.revoke(choice))
                }
                fixture.workers.single().released.complete(Unit)
                controller.newIntent(Intent(UsbManager.ACTION_USB_ACCESSORY_ATTACHED))
                shadowOf(Looper.getMainLooper()).idle()
                assertEquals("T1405 repeated action cannot admit a new worker", 1, fixture.workers.size)
                assertFalse(fixture.binding.state.value.reconnectAllowed)
                controller.get().configuredConnection.connect()
                assertEquals("Manual Connect remains available", 2, fixture.workers.size)
            } finally { controller.destroy(); CameraOwner.reset(); AudioOwner.reset() }
        }
    }
    @Test fun t1405SuppliedAccessoryExtraCannotReplaceNativePresence() {
        ConnectionUiFixture().use { fixture ->
            WorkerTestBridge.install()
            Prefs(RuntimeEnvironment.getApplication()).checkUpdates = false
            fixture.nativeAttachment.absent()
            val forged = Intent(UsbManager.ACTION_USB_ACCESSORY_ATTACHED)
                .putExtra(UsbManager.EXTRA_ACCESSORY, nativeAccessory())
            val controller = Robolectric.buildActivity(MainActivity::class.java, forged).create()
            try {
                assertTrue(fixture.workers.isEmpty())
                fixture.nativeAttachment.reattach()
                controller.newIntent(Intent(UsbManager.ACTION_USB_ACCESSORY_ATTACHED))
                assertEquals(1, fixture.workers.size)
            } finally { controller.destroy(); CameraOwner.reset(); AudioOwner.reset() }
        }
    }

}
