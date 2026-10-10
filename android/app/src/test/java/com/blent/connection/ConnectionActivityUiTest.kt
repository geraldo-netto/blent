package com.blent.connection

import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.AndroidComposeTestRule
import com.blent.*
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.Robolectric
import org.robolectric.RobolectricTestRunner
import org.robolectric.RuntimeEnvironment
import org.robolectric.annotation.Config
import org.robolectric.annotation.LooperMode
import org.robolectric.util.ReflectionHelpers

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [27, 34], qualifiers = "w960dp-h900dp-land", instrumentedPackages = ["androidx.compose.ui.platform"])
@LooperMode(LooperMode.Mode.PAUSED)
class ConnectionActivityUiTest {
    class Fixture : org.junit.rules.ExternalResource() {
        internal lateinit var usb: ConnectionUiFixture
        lateinit var controller: org.robolectric.android.controller.ActivityController<MainActivity>
        override fun before() {
            usb = ConnectionUiFixture()
            WorkerTestBridge.install(); CameraOwner.reset(); AudioOwner.reset()
            val prefs = Prefs(RuntimeEnvironment.getApplication()).apply { checkUpdates = false; hostToken = null }
            controller = Robolectric.buildActivity(MainActivity::class.java).create()
            controller.get().session.stop()
            ReflectionHelpers.setField(controller.get(), "session", SessionCoordinator(prefs, { it() }, null, null))
            controller.start().resume().visible()
        }
        override fun after() {
            val content = controller.get().findViewById<android.view.ViewGroup>(android.R.id.content)
            (content.getChildAt(0) as? androidx.compose.ui.platform.AbstractComposeView)?.disposeComposition()
            controller.pause().stop().destroy()
            CameraOwner.reset(); AudioOwner.reset(); usb.close()
        }
    }
    private val fixture = Fixture()
    @get:Rule val compose = AndroidComposeTestRule(fixture) { it.controller.get() }

    @Test fun t893PairingPresentationFollowsForegroundWithoutRetiringItsOwner() {
        compose.onNodeWithText("⚙").performClick()
        compose.onNodeWithText("Connect USB").performScrollTo().performClick()
        compose.runOnIdle { fixture.usb.prompt() }
        compose.onNodeWithText("Pair with this computer?").assertExists()
        compose.runOnIdle { fixture.controller.pause().stop() }
        compose.onNodeWithText("Pair with this computer?").assertDoesNotExist()
        compose.runOnIdle {
            assertTrue(fixture.usb.binding.state.value.active)
            assertEquals(0, fixture.usb.workers.single().cancelled)
            fixture.controller.start().resume().visible()
        }
        compose.onNodeWithText("Pair with this computer?").assertExists()
        compose.onNodeWithText("Approve computer").performClick()
        compose.runOnIdle {
            assertEquals(listOf(ConnectionWorkerCommand.Decide(fixture.usb.ticket, true, false)), fixture.usb.workers.single().commands)
            assertNull(compose.activity.cameras.selected)
            assertFalse(compose.activity.microphone.sharing)
        }
    }
}
