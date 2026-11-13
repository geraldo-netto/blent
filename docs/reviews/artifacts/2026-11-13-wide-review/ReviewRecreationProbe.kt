package com.blent.connection

import android.content.Intent
import android.hardware.usb.UsbManager
import android.os.Bundle
import com.blent.*
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.Robolectric
import org.robolectric.RuntimeEnvironment
import org.robolectric.annotation.Config

@RunWith(com.blent.ComposeFixtureRunner::class)
@Config(sdk = [27, 34], instrumentedPackages = ["androidx.compose.ui.platform"])
class ReviewRecreationProbe {
    @Test fun t1396RestoredAttachmentIntentMustNotUndoExplicitDisconnect() =
        restored(UsbManager.ACTION_USB_ACCESSORY_ATTACHED, false, 1)

    @Test fun t1396OrdinaryLauncherRestorationKeepsDisconnect() =
        restored(Intent.ACTION_MAIN, false, 1)

    @Test fun t1396FreshAttachmentStillStartsAnotherConnection() =
        restored(Intent.ACTION_MAIN, true, 2)

    private fun restored(action: String, fresh: Boolean, expected: Int) {
        ConnectionUiFixture().use { fixture ->
            WorkerTestBridge.install()
            Prefs(RuntimeEnvironment.getApplication()).checkUpdates = false
            val launch = Intent(UsbManager.ACTION_USB_ACCESSORY_ATTACHED)
            val first = Robolectric.buildActivity(MainActivity::class.java, launch).create()
            val saved = Bundle()
            try {
                assertEquals(1, fixture.workers.size)
                first.get().configuredConnection.disconnect()
                fixture.workers.single().released.complete(Unit)
                assertFalse(fixture.binding.state.value.reconnectAllowed)
                first.saveInstanceState(saved)
            } finally { first.destroy() }
            val restored = Robolectric.buildActivity(MainActivity::class.java, Intent(action)).create(saved)
            try {
                if (fresh) restored.newIntent(Intent(UsbManager.ACTION_USB_ACCESSORY_ATTACHED))
                println("RECREATION workers=${fixture.workers.size}, reconnectAllowed=${fixture.binding.state.value.reconnectAllowed}")
                assertEquals("Restored launch intent is not a new attachment", expected, fixture.workers.size)
            } finally {
                restored.destroy(); CameraOwner.reset(); AudioOwner.reset()
                org.robolectric.Shadows.shadowOf(android.os.Looper.getMainLooper()).idle()
            }
        }
    }
}
