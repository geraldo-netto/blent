package com.blent.connection

import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.AndroidComposeTestRule
import com.blent.MainActivity
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.Robolectric
import org.robolectric.annotation.Config
import org.robolectric.annotation.LooperMode

@RunWith(com.blent.ComposeFixtureRunner::class)
@Config(sdk = [27, 34], qualifiers = "w960dp-h900dp-land", instrumentedPackages = ["androidx.compose.ui.platform"])
@LooperMode(LooperMode.Mode.PAUSED)
class SavedComputerActivityTest {
    private val fixture = ConnectionActivityUiTest.Fixture()
    @get:Rule val compose = AndroidComposeTestRule(fixture) { fixture.controller.get() }
    private fun open() {
        compose.onNodeWithText("⚙").performClick()
        compose.onNodeWithText("Saved computers").performScrollTo().performClick()
    }
    private fun replaceActivity() {
        compose.runOnIdle {
            val content = fixture.controller.get().findViewById<android.view.ViewGroup>(android.R.id.content)
            (content.getChildAt(0) as? androidx.compose.ui.platform.AbstractComposeView)?.disposeComposition()
            fixture.controller.pause().stop().destroy()
            assertNull(fixture.usb.saved.state.value.viewer)
            fixture.controller = Robolectric.buildActivity(MainActivity::class.java).create().start().resume().visible()
            assertSame(fixture.usb.saved, fixture.controller.get().savedComputers)
        }
    }
    private fun select() { compose.onNodeWithText("Forget ${savedPeer(1).fingerprint}").performClick() }

    @Test fun t1169ActivityReplacementRetainsAcceptedForgetWithoutDuplicateRequestOrMediaConsent() {
        open(); compose.runOnIdle { fixture.usb.savedRuns.single().complete() }; select()
        compose.onNodeWithText("Confirm forget computer").performClick()
        replaceActivity(); open()
        compose.onNodeWithText("Reading or updating saved computers…").assertExists()
        compose.runOnIdle {
            assertEquals(2, fixture.usb.savedRequests.size)
            assertEquals(0, fixture.usb.savedRuns.last().cancelled)
            assertTrue(fixture.usb.workers.isEmpty())
            assertNull(compose.activity.cameras.selected); assertFalse(compose.activity.microphone.sharing)
            fixture.usb.savedRuns.last().complete(0)
        }
        compose.onNodeWithText("No saved computers.").assertExists()
    }
    @Test fun t1169ActivityReplacementRevokesOldConfirmationEvenForIdenticalStoredFingerprint() {
        open(); compose.runOnIdle { fixture.usb.savedRuns.single().complete() }; select()
        val oldViewer = compose.runOnIdle { checkNotNull(fixture.usb.saved.state.value.viewer) }
        val oldChoice = compose.runOnIdle { checkNotNull(fixture.usb.saved.choice(oldViewer, savedPeer(1))) }
        replaceActivity(); open()
        compose.onNodeWithText("Forget this computer?").assertDoesNotExist()
        compose.runOnIdle {
            assertFalse(fixture.usb.saved.forget(oldViewer, oldChoice))
            assertEquals(1, fixture.usb.savedRequests.size)
        }
        select(); compose.onNodeWithText("Confirm forget computer").performClick()
        compose.runOnIdle {
            assertEquals(2, fixture.usb.savedRequests.size); fixture.usb.savedRuns.last().complete(0)
        }
        compose.onNodeWithText("No saved computers.").assertExists()
    }
}
