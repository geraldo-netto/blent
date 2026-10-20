package com.blent.connection

import androidx.compose.foundation.layout.Column
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.createComposeRule
import com.blent.BlentTheme
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.annotation.Config
import org.robolectric.annotation.LooperMode

@OptIn(ExperimentalTestApi::class)
@RunWith(com.blent.ComposeFixtureRunner::class)
// T1076/T1171: ordinary controls own their dispatcher sandbox. Actual
// Activity teardown retains its separate full Compose platform instrumentation.
@Config(sdk = [27, 34], qualifiers = "w960dp-h900dp-land")
@LooperMode(LooperMode.Mode.PAUSED)
class ConnectionControlsTest {
    @get:Rule val compose = createComposeRule()

    private fun render(fixture: ConnectionUiFixture) {
        val configured = ConfiguredConnection(fixture.binding,
            ConnectionPreferenceStore(PreferenceMemoryStorage(), kotlinx.coroutines.Dispatchers.Unconfined), fixture.scope)
        compose.setContent { BlentTheme { Column {
            ConnectionControls(fixture.binding, configured, fixture.saved)
            ConnectionPairingDialog(fixture.binding)
        } } }
    }

    @Test fun t893ExplicitApprovalUsesMatchingLabelsAndDurableTrustBeforeRevoke() {
        ConnectionUiFixture().use { fixture ->
            render(fixture)
            compose.onNodeWithText("Connect USB").assertIsEnabled().performClick()
            compose.onNodeWithText("Connect USB").assertIsNotEnabled()
            compose.runOnIdle { assertEquals(1, fixture.workers.size); fixture.prompt() }
            compose.onNodeWithText("Pair with this computer?").assertExists()
            compose.onAllNodesWithText("Computer fingerprint: ${fixture.computer.fingerprint}").onLast().assertExists()
            compose.onAllNodesWithText("Tablet fingerprint: ${fixture.tablet.fingerprint}").onLast().assertExists()
            compose.onNodeWithContentDescription("Remember this computer").assertIsOff().performClick()
            compose.onNodeWithText("Approve computer").performClick()
            compose.runOnIdle {
                assertEquals(listOf(ConnectionWorkerCommand.Decide(fixture.ticket, true, true)), fixture.workers.single().commands)
            }
            compose.onNodeWithText("Computer trust is not saved.").assertExists()
            compose.runOnIdle { fixture.workers.single().report(ConnectionWorkerEvent.Trust(fixture.computer, true)) }
            compose.onNodeWithText("Computer trust saved.").assertExists()
            compose.onNodeWithText("Revoke computer").performClick()
            compose.runOnIdle {
                assertEquals(ConnectionWorkerCommand.Revoke(fixture.computer), fixture.workers.single().commands.last())
            }
            compose.onNodeWithText("Revoke computer").assertIsNotEnabled()
            compose.onNodeWithText("Computer trust saved.").assertExists()
            compose.runOnIdle { fixture.workers.single().report(ConnectionWorkerEvent.Trust(fixture.computer, false)) }
            compose.onNodeWithText("Computer trust is not saved.").assertExists()
        }
    }

    @Test fun t893DenialAndDisconnectRemainExplicitAndCheckboxResetsOnNewChoice() {
        ConnectionUiFixture().use { fixture ->
            render(fixture)
            compose.onNodeWithText("Connect USB").performClick()
            compose.runOnIdle { fixture.prompt() }
            compose.onNodeWithContentDescription("Remember this computer").performClick()
            compose.onNodeWithText("Deny").performClick()
            compose.runOnIdle {
                assertEquals(listOf(ConnectionWorkerCommand.Decide(fixture.ticket, false, false)), fixture.workers.single().commands)
                fixture.workers.single().released.complete(Unit)
            }
            compose.onNodeWithText("Connect USB").performClick()
            compose.runOnIdle { fixture.prompt() }
            compose.onNodeWithContentDescription("Remember this computer").assertIsOff()
            compose.onNodeWithText("Approve computer").performClick()
            compose.onNodeWithText("Disconnect USB").performClick()
            compose.onNodeWithText("Closing USB resources…").assertExists()
            compose.onNodeWithText("Disconnect USB").assertIsNotEnabled()
            compose.runOnIdle {
                assertEquals(ConnectionWorkerCommand.Decide(fixture.ticket, true, false), fixture.workers.last().commands.single())
                assertEquals(1, fixture.workers.last().cancelled)
            }
        }
    }

    @Test
    @Config(qualifiers = "w640dp-h360dp-land")
    fun t893SmallWindowKeepsRememberChoiceReachable() {
        ConnectionUiFixture().use { fixture ->
            render(fixture)
            compose.onNodeWithText("Connect USB").performClick()
            compose.runOnIdle { fixture.prompt() }
            compose.onNodeWithContentDescription("Remember this computer").performScrollTo().assertIsDisplayed()
            compose.onNodeWithText("Approve computer").assertIsDisplayed()
            compose.onNodeWithText("Deny").performClick()
        }
    }

    // T1072: exercise the native legacy Back callback, preserving its dismissal assertion.
    @Suppress("DEPRECATION")
    @Test fun t893DismissingPairingNeverApproves() {
        ConnectionUiFixture().use { fixture ->
            render(fixture)
            compose.onNodeWithText("Connect USB").performClick()
            compose.runOnIdle { fixture.prompt() }
            compose.runOnIdle { org.robolectric.shadows.ShadowDialog.getLatestDialog().onBackPressed() }
            compose.runOnIdle {
                assertEquals(listOf(ConnectionWorkerCommand.Decide(fixture.ticket, false, false)), fixture.workers.single().commands)
            }
        }
    }
}
