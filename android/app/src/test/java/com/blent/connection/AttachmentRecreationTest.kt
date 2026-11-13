package com.blent.connection

import android.content.Intent
import android.hardware.usb.UsbManager
import android.os.Bundle
import android.os.Looper
import com.blent.*
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.Robolectric
import org.robolectric.RuntimeEnvironment
import org.robolectric.Shadows
import org.robolectric.annotation.Config

@RunWith(com.blent.ComposeFixtureRunner::class)
@Config(sdk = [27, 34], instrumentedPackages = ["androidx.compose.ui.platform"])
class AttachmentRecreationTest {
    private enum class Cutoff { DISCONNECT, DENY, REVOKE }
    private enum class Restart { NONE, ATTACHMENT, MANUAL }

    @Test fun t1396RestoredAttachmentKeepsDisconnect() = restored(Cutoff.DISCONNECT)
    @Test fun t1396RestoredAttachmentKeepsDenial() = restored(Cutoff.DENY)
    @Test fun t1396RestoredAttachmentKeepsRevocation() = restored(Cutoff.REVOKE)
    @Test fun t1396OrdinaryLauncherRestorationKeepsCutoff() =
        restored(Cutoff.DISCONNECT, Intent.ACTION_MAIN)
    @Test fun t1396FreshAttachmentStillStartsAfterRetirement() =
        restored(Cutoff.DISCONNECT, Intent.ACTION_MAIN, Restart.ATTACHMENT)
    @Test fun t1396ExplicitConnectStillStartsAfterRestoration() =
        restored(Cutoff.DISCONNECT, Intent.ACTION_MAIN, Restart.MANUAL)

    private fun stop(activity: MainActivity, fixture: ConnectionUiFixture, cutoff: Cutoff) {
        fixture.prompt()
        val choice = checkNotNull(fixture.binding.state.value.peer)
        when (cutoff) {
            Cutoff.DISCONNECT -> activity.configuredConnection.disconnect()
            Cutoff.DENY -> assertTrue(fixture.binding.decide(choice, false, false))
            Cutoff.REVOKE -> assertTrue(fixture.binding.revoke(choice))
        }
        fixture.workers.single().released.complete(kotlin.Unit)
        assertFalse(fixture.binding.state.value.reconnectAllowed)
        assertFalse(fixture.binding.state.value.active)
    }

    private fun restored(cutoff: Cutoff, action: String = UsbManager.ACTION_USB_ACCESSORY_ATTACHED,
        restart: Restart = Restart.NONE) {
        ConnectionUiFixture().use { fixture ->
            WorkerTestBridge.install()
            Prefs(RuntimeEnvironment.getApplication()).checkUpdates = false
            val launch = Intent(UsbManager.ACTION_USB_ACCESSORY_ATTACHED)
            val first = Robolectric.buildActivity(MainActivity::class.java, launch).create()
            val saved = Bundle()
            try {
                assertEquals(1, fixture.workers.size)
                stop(first.get(), fixture, cutoff)
                first.saveInstanceState(saved)
            } finally { first.destroy() }
            val restored = Robolectric.buildActivity(MainActivity::class.java, Intent(action)).create(saved)
            try {
                assertEquals("T1396 restored Intent cannot create a worker", 1, fixture.workers.size)
                assertFalse("T1396 restored Intent cannot lift cutoff", fixture.binding.state.value.reconnectAllowed)
                when (restart) {
                    Restart.ATTACHMENT -> {
                        fixture.nativeAttachment.reattach()
                        restored.newIntent(Intent(UsbManager.ACTION_USB_ACCESSORY_ATTACHED))
                    }
                    Restart.MANUAL -> restored.get().configuredConnection.connect()
                    Restart.NONE -> Unit
                }
                assertEquals(if (restart == Restart.NONE) 1 else 2, fixture.workers.size)
            } finally {
                restored.destroy(); CameraOwner.reset(); AudioOwner.reset()
                Shadows.shadowOf(Looper.getMainLooper()).idle()
            }
        }
    }
}
