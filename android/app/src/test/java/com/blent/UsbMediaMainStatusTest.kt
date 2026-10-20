package com.blent

import android.os.Looper
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.createComposeRule
import com.blent.connection.*
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.Robolectric
import org.robolectric.Shadows
import org.robolectric.annotation.Config
import org.robolectric.annotation.LooperMode

@RunWith(ComposeFixtureRunner::class)
@Config(sdk = [27, 34], qualifiers = "w960dp-h600dp-land",
    instrumentedPackages = ["androidx.compose.ui.platform"])
@LooperMode(LooperMode.Mode.PAUSED)
class UsbMediaMainStatusTest {
    @get:Rule val compose = createComposeRule()

    @Test fun t1233AuthenticatedMediaShowsControlsWithoutStartingCameraOrDisplay() {
        WorkerTestBridge.install(); CameraOwner.reset(); AudioOwner.reset()
        ConnectionUiFixture().use { fixture ->
            val controller = Robolectric.buildActivity(MainActivity::class.java).setup().visible()
            try {
                fixture.connect(); fixture.prompt()
                val worker = fixture.workers.single()
                worker.report(ConnectionWorkerEvent.State(ConnectionWorkerPhase.AUTHENTICATED))
                worker.report(ConnectionWorkerEvent.CameraOffered(LocalCameraOffer(cameraLease(), cameraProfile(), false)))
                Shadows.shadowOf(Looper.getMainLooper()).idle()
                compose.onNodeWithText("USB media connected").assertIsDisplayed()
                compose.onNodeWithText("Waiting for your computer…").assertDoesNotExist()
                compose.onNodeWithText("Virtual desktop is not streaming.", substring = true).assertIsDisplayed()
                compose.onNodeWithText("Allow USB debugging", substring = true).assertDoesNotExist()
                compose.onNodeWithText("Open camera and audio controls").performScrollTo().performClick()
                compose.onNodeWithText("USB camera request: Rear.").performScrollTo().assertIsDisplayed()
                compose.onNodeWithText("Start camera").performScrollTo().assertIsEnabled()
                compose.runOnIdle {
                    assertTrue("T1233 opening controls must not grant media", worker.commands.isEmpty())
                    assertNull(controller.get().connection.state.value.display)
                    assertFalse(controller.get().session.presentation.connected)
                }
                // T1233: closing the borrowed controls cannot disconnect media.
                compose.onNodeWithText("Close camera and audio controls").performScrollTo().performClick()
                compose.onNodeWithText("Camera and audio").assertDoesNotExist()
                compose.onNodeWithText("USB media connected").assertIsDisplayed()
                compose.runOnIdle { assertTrue(worker.commands.isEmpty()) }
            } finally { controller.pause().stop().destroy(); CameraOwner.reset(); AudioOwner.reset() }
        }
    }

    @Test fun t1233ClosingAndReplacementConnectionCannotRetainAuthenticatedStatus() {
        WorkerTestBridge.install(); CameraOwner.reset(); AudioOwner.reset()
        ConnectionUiFixture().use { fixture ->
            val controller = Robolectric.buildActivity(MainActivity::class.java).setup().visible()
            try {
                compose.onNodeWithText("Waiting for your computer…").assertIsDisplayed()
                fixture.connect(); fixture.prompt()
                val original = fixture.workers.single()
                original.report(ConnectionWorkerEvent.State(ConnectionWorkerPhase.AUTHENTICATED))
                Shadows.shadowOf(Looper.getMainLooper()).idle()
                compose.onNodeWithText("USB media connected").assertIsDisplayed()
                original.report(ConnectionWorkerEvent.State(ConnectionWorkerPhase.CLOSING))
                compose.onNodeWithText("USB media connected").assertDoesNotExist()
                compose.onNodeWithText("Waiting for your computer…").assertIsDisplayed()
                fixture.binding.disconnect(); original.released.complete(Unit)
                fixture.connect(); fixture.prompt()
                compose.onNodeWithText("USB media connected").assertDoesNotExist()
                original.report(ConnectionWorkerEvent.State(ConnectionWorkerPhase.AUTHENTICATED))
                compose.onNodeWithText("USB media connected").assertDoesNotExist()
                fixture.workers.last().report(ConnectionWorkerEvent.State(ConnectionWorkerPhase.AUTHENTICATED))
                compose.onNodeWithText("USB media connected").assertIsDisplayed()
            } finally { controller.pause().stop().destroy(); CameraOwner.reset(); AudioOwner.reset() }
        }
    }
}
