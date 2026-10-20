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
class ConnectionActivityTest {
    @Test fun t1099SavedLimitsFollowAccessoryLaunchAndNextAttachAcrossRecreation() = kotlinx.coroutines.runBlocking {
        ConnectionUiFixture().use { fixture ->
            WorkerTestBridge.install()
            try {
                Prefs(RuntimeEnvironment.getApplication()).apply { checkUpdates = false; hostToken = null }
                val original = ConnectionWorkerSpec("usb", ConnectionSessionLimits(4096, 8, 1024, 2000),
                    ConnectionRunLimits(1234, 2345), 3000, 4000, 2, 3, 250)
                assertTrue(fixture.preferences.save(ConnectionPreferences.fields(original)).isSuccess)
                val first = Robolectric.buildActivity(MainActivity::class.java, Intent(UsbManager.ACTION_USB_ACCESSORY_ATTACHED)).create()
                try { assertEquals(listOf(original), fixture.specifications) }
                finally { first.destroy() }
                val replacement = Robolectric.buildActivity(MainActivity::class.java).create()
                try {
                    val next = original.copy(timerMs = 1000)
                    assertTrue(fixture.preferences.save(ConnectionPreferences.fields(next)).isSuccess)
                    replacement.newIntent(Intent(UsbManager.ACTION_USB_ACCESSORY_ATTACHED))
                    assertEquals(listOf(original), fixture.specifications)
                    assertEquals(0, fixture.workers.single().cancelled)
                    replacement.get().configuredConnection.disconnect()
                    fixture.workers.single().released.complete(Unit)
                    replacement.newIntent(Intent(UsbManager.ACTION_USB_ACCESSORY_ATTACHED))
                    assertEquals(listOf(original, next), fixture.specifications)
                    assertTrue(fixture.workers.last().commands.isEmpty())
                } finally { replacement.destroy() }
            } finally { CameraOwner.reset(); AudioOwner.reset() }
        }
    }

    @Test fun t893AccessoryIntentCannotApplyUnrelatedLegacyCredentials() {
        val fixture = ConnectionUiFixture()
        WorkerTestBridge.install()
        val prefs = Prefs(RuntimeEnvironment.getApplication()).apply { checkUpdates = false; hostToken = "old-token" }
        val controller = Robolectric.buildActivity(MainActivity::class.java).create()
        val receiver = VideoReceiver { error("no network in T893") }
        try {
            org.robolectric.util.ReflectionHelpers.setField(controller.get(), "session", SessionCoordinator(prefs, { it() }, receiver, null))
            assertEquals("old-token", receiver.token)
            prefs.hostToken = "new-token"
            controller.newIntent(Intent(UsbManager.ACTION_USB_ACCESSORY_ATTACHED))
            assertEquals("T893 accessory attach must not reroute legacy credentials", "old-token", receiver.token)
            assertEquals(1, fixture.workers.size)
            controller.newIntent(Intent(UsbManager.ACTION_USB_ACCESSORY_ATTACHED))
            assertEquals(1, fixture.workers.size)
            controller.newIntent(Intent(Intent.ACTION_MAIN))
            assertEquals("new-token", receiver.token)
        } finally { controller.destroy(); CameraOwner.reset(); AudioOwner.reset(); fixture.close() }
    }
    @Test fun t893AccessoryLaunchAndActivityRecreationRetainTheSameConnectionOwner() {
        ConnectionUiFixture().use { fixture ->
            WorkerTestBridge.install()
            Prefs(RuntimeEnvironment.getApplication()).checkUpdates = false
            val first = Robolectric.buildActivity(MainActivity::class.java, Intent(UsbManager.ACTION_USB_ACCESSORY_ATTACHED)).create()
            assertSame(fixture.binding, first.get().connection)
            assertEquals(1, fixture.workers.size)
            val worker = fixture.workers.single()
            fixture.prompt()
            first.destroy()
            val second = Robolectric.buildActivity(MainActivity::class.java).create()
            try {
                assertSame(fixture.binding, second.get().connection)
                assertEquals(1, fixture.workers.size)
                assertEquals(0, worker.cancelled)
                assertNotNull(second.get().connection.state.value.approval)
                assertTrue(second.get().connection.decide(checkNotNull(second.get().connection.state.value.approval), false, false))
                assertEquals(listOf(ConnectionWorkerCommand.Decide(fixture.ticket, false, false)), worker.commands)
            } finally { second.destroy(); CameraOwner.reset(); AudioOwner.reset() }
        }
    }
}
