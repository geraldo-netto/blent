package com.blent

import android.os.Looper
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.createComposeRule
import com.blent.connection.*
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.*
import org.robolectric.annotation.Config
import org.robolectric.annotation.LooperMode

@RunWith(com.blent.ComposeFixtureRunner::class)
@Config(sdk = [27, 34], qualifiers = "w960dp-h600dp-land",
    instrumentedPackages = ["androidx.compose.ui.platform"])
@LooperMode(LooperMode.Mode.PAUSED)
class CameraConsentMainActivityTest {
    @get:Rule val compose = createComposeRule()

    @Test fun t973ProductionSettingsSendOriginalCameraChoiceWithoutDisplayOrLegacyCapture() {
        WorkerTestBridge.install()
        CameraOwner.reset()
        ConnectionUiFixture().use { fixture ->
            val controller = Robolectric.buildActivity(MainActivity::class.java).setup().visible()
            try {
                fixture.connect(); fixture.prompt()
                val worker = fixture.workers.last()
                worker.report(ConnectionWorkerEvent.State(ConnectionWorkerPhase.AUTHENTICATED))
                worker.report(ConnectionWorkerEvent.CameraOffered(LocalCameraOffer(cameraLease(), cameraProfile(), false)))
                Shadows.shadowOf(Looper.getMainLooper()).idle()
                compose.onNodeWithContentDescription("Open settings").performClick()
                compose.onNodeWithText("USB camera request: Rear.").performScrollTo().assertExists()
                compose.onNodeWithText("Start camera").performScrollTo().assertIsEnabled().performClick()
                compose.runOnIdle {
                    assertEquals(1, worker.commands.filterIsInstance<ConnectionWorkerCommand.CameraChoice>().size)
                    assertTrue(worker.commands.filterIsInstance<ConnectionWorkerCommand.CameraChoice>().single().choice.enabled)
                    assertNull(controller.get().cameras.selected)
                    assertFalse(controller.get().connection.state.value.display != null)
                }
                compose.onNodeWithText("Stop camera").performClick()
                compose.runOnIdle {
                    assertFalse(worker.commands.filterIsInstance<ConnectionWorkerCommand.CameraChoice>().last().choice.enabled)
                    worker.report(ConnectionWorkerEvent.CameraClosed(cameraLease()))
                    assertTrue(fixture.binding.state.value.authenticated)
                }
                compose.onNodeWithText("Start camera").assertIsNotEnabled()
            } finally { controller.pause().stop().destroy(); CameraOwner.reset() }
        }
    }
}
