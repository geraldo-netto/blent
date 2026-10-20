package com.blent.connection

import androidx.compose.foundation.layout.Column
import androidx.compose.ui.semantics.SemanticsActions
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
@Config(sdk = [27, 34], qualifiers = "w960dp-h900dp-land")
@LooperMode(LooperMode.Mode.PAUSED)
class SavedBrowserOwnershipTest {
    @get:Rule val compose = createComposeRule()
    private fun render(f: ConnectionUiFixture) {
        compose.setContent { BlentTheme { Column { SavedComputerControls(f.saved) } } }
    }
    private fun open() { compose.onNodeWithText("Saved computers").performClick() }

    @Test fun t1169ExpiredBrowserCannotControlCurrentWorkAndExplicitReopenRenewsItsView() {
        ConnectionUiFixture().use { f ->
            render(f); open()
            val current = compose.runOnIdle { f.saved.attach() }
            try {
                compose.onNodeWithText("This saved-computer view is no longer current. Close and reopen it.").assertExists()
                compose.onNodeWithText("Stop saved-computer access").assertIsNotEnabled()
                compose.runOnIdle { assertTrue(current.current); f.savedRuns.single().complete() }
                compose.onNodeWithText("Refresh saved computers").assertIsNotEnabled()
                compose.onNodeWithText("Forget ${savedPeer(1).fingerprint}").assertIsNotEnabled()
            } finally { compose.runOnIdle { current.close() } }
            compose.onNodeWithText("Close saved computers").performClick(); open()
            compose.onNodeWithText("Refresh saved computers").assertIsEnabled()
            compose.onNodeWithText("Forget ${savedPeer(1).fingerprint}").assertIsEnabled()
            compose.runOnIdle { assertEquals(1, f.savedRequests.size) }
        }
    }

    @Test fun t1169OldBrowserCloseCannotRevokeAReplacementBrowser() {
        ConnectionUiFixture().use { f ->
            render(f); open(); compose.runOnIdle { f.savedRuns.single().complete() }
            val stale = checkNotNull(compose.onNodeWithText("Close saved computers")
                .fetchSemanticsNode().config[SemanticsActions.OnClick].action)
            compose.onNodeWithText("Close saved computers").performClick(); open()
            val viewer = compose.runOnIdle { checkNotNull(f.saved.state.value.viewer) }
            compose.runOnIdle { stale(); assertSame(viewer, f.saved.state.value.viewer) }
            compose.onNodeWithText("Refresh saved computers").assertIsEnabled()
        }
    }
}
