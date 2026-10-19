package com.blent.connection

import androidx.compose.foundation.layout.Column
import androidx.compose.ui.semantics.SemanticsProperties
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.createComposeRule
import com.blent.BlentTheme
import kotlinx.coroutines.*
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config
import org.robolectric.annotation.LooperMode

@OptIn(ExperimentalTestApi::class)
@RunWith(RobolectricTestRunner::class)
@Config(sdk = [27, 34], qualifiers = "w960dp-h900dp-land")
@LooperMode(LooperMode.Mode.PAUSED)
class ConnectionPreferenceControlsTest {
    @get:Rule val compose = createComposeRule()
    private fun render(fixture: ConnectionUiFixture, store: ConnectionPreferenceStore): ConfiguredConnection {
        val configured = ConfiguredConnection(fixture.binding, store, fixture.scope)
        compose.setContent { BlentTheme { Column { ConnectionControls(fixture.binding, configured) } } }
        return configured
    }
    private fun open() { compose.onNodeWithText("USB capacity and deadlines").performClick() }
    private fun edit(label: String, value: String) {
        compose.onNodeWithText(label).performScrollTo().performTextReplacement(value)
    }

    @Test fun t1099SavedControlsAffectOnlyNextWorkerAndLeaveActiveMediaOwnerUntouched() {
        ConnectionUiFixture().use { fixture ->
            render(fixture, ConnectionPreferenceStore(PreferenceMemoryStorage(), Dispatchers.Unconfined))
            compose.onNodeWithText("Connect USB").performClick()
            open()
            edit("Idle fallback (ms)", "250")
            compose.onNodeWithText("Save USB settings").performClick()
            compose.onNodeWithText("Saved for the next USB connection.").assertExists()
            compose.onNodeWithText("Close settings").performClick()
            compose.runOnIdle {
                assertEquals(listOf(ConnectionWorkerSpec("usb")), fixture.specifications)
                assertEquals(0, fixture.workers.single().cancelled)
                assertTrue(fixture.workers.single().commands.isEmpty())
            }
            compose.onNodeWithText("Disconnect USB").performClick()
            compose.runOnIdle { fixture.workers.single().released.complete(Unit) }
            compose.onNodeWithText("Connect USB").performClick()
            compose.runOnIdle {
                assertEquals(listOf(ConnectionWorkerSpec("usb"), ConnectionWorkerSpec("usb", timerMs = 250)), fixture.specifications)
            }
        }
    }

    @Test fun t1099InvalidAndFailedSaveAreVisibleAndNeverSilentlyConfigureConnection() {
        ConnectionUiFixture().use { fixture ->
            val storage = PreferenceMemoryStorage().also { it.accepted = false }
            render(fixture, ConnectionPreferenceStore(storage, Dispatchers.Unconfined))
            open(); edit("Channel buffer (bytes)", "0")
            compose.onNodeWithText("Channel buffer (bytes)").assert(SemanticsMatcher.keyIsDefined(SemanticsProperties.Error))
            compose.onNodeWithText("Save USB settings").performClick()
            compose.runOnIdle { assertEquals(0, storage.writes) }
            edit("Channel buffer (bytes)", "4096")
            compose.onNodeWithText("Save USB settings").performClick()
            compose.onNodeWithText("USB settings could not be saved.").assertExists()
            compose.onNodeWithText("Close settings").performClick()
            compose.onNodeWithText("Connect USB").performClick()
            compose.onNodeWithText("USB settings save was not confirmed. Save settings again before connecting.").assertExists()
            compose.runOnIdle { assertTrue(fixture.workers.isEmpty()); storage.accepted = true }
            open()
            compose.onNodeWithText("Use defaults").performScrollTo().performClick()
            compose.onNodeWithText("Save USB settings").performClick()
            compose.onNodeWithText("Saved for the next USB connection.").assertExists()
            compose.onNodeWithText("Close settings").performClick()
            compose.onNodeWithText("Connect USB").performClick()
            compose.runOnIdle { assertEquals(listOf(ConnectionWorkerSpec("usb")), fixture.specifications) }
        }
    }

    @Suppress("DEPRECATION")
    @Test @Config(qualifiers = "w640dp-h360dp-land")
    fun t1099SmallWindowReloadValidationAndNativeBackKeepActionsReachable() {
        ConnectionUiFixture().use { fixture ->
            render(fixture, ConnectionPreferenceStore(PreferenceMemoryStorage(), Dispatchers.Unconfined))
            open()
            compose.onNodeWithText("Save USB settings").assertIsDisplayed()
            edit("Idle fallback (ms)", "333")
            compose.onNodeWithText("Reload saved settings").performScrollTo().performClick()
            compose.onNodeWithText("Idle fallback (ms)").assertTextContains("100")
            edit("Idle fallback (ms)", "1".repeat(21))
            compose.onNodeWithText("Use at most seven decimal digits.").assertExists()
            compose.runOnIdle { org.robolectric.shadows.ShadowDialog.getLatestDialog().onBackPressed() }
            compose.onNodeWithText("USB connection settings").assertDoesNotExist()
            open()
            compose.onNodeWithText("Idle fallback (ms)").assertTextContains("100")
            compose.onNodeWithText("Close settings").performClick()
        }
    }

    @Suppress("DEPRECATION")
    @Test fun t1099PendingStartAndClosedSettingsRetainOriginalStorageUntilSettlement() {
        ConnectionUiFixture().use { fixture ->
            val dispatcher = ConnectionActorScheduler()
            val configured = render(fixture, ConnectionPreferenceStore(PreferenceMemoryStorage(), dispatcher))
            try {
                compose.onNodeWithText("Connect USB").performClick()
                compose.onNodeWithText("Loading USB connection settings…").assertExists()
                compose.onNodeWithText("Connect USB").assertIsNotEnabled()
                compose.onNodeWithText("Disconnect USB").performClick()
                compose.runOnIdle { dispatcher.runCurrent() }
                compose.runOnIdle { assertTrue(fixture.workers.isEmpty()) }
                open()
                compose.onNodeWithText("Waiting for USB settings storage…").assertExists()
                compose.onNodeWithText("Save USB settings").assertIsNotEnabled()
                compose.onNodeWithText("Close settings").performClick()
                compose.runOnIdle { dispatcher.runCurrent() }
                compose.onNodeWithText("Connect USB").performClick()
                compose.runOnIdle { dispatcher.runCurrent() }
                compose.runOnIdle { assertEquals(1, fixture.workers.size) }
            } finally {
                compose.runOnIdle {
                    org.robolectric.shadows.ShadowDialog.getLatestDialog()?.onBackPressed()
                    configured.disconnect(); dispatcher.runCurrent()
                }
                compose.waitForIdle()
                dispatcher.runCurrent()
            }
            assertTrue(dispatcher.empty())
        }
    }
}
