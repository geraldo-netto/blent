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

@RunWith(com.blent.ComposeFixtureRunner::class)
@Config(sdk = [27, 34], qualifiers = "w960dp-h900dp-land")
@LooperMode(LooperMode.Mode.PAUSED)
class ConnectionRetryControlsTest {
    @get:Rule val compose = createComposeRule()

    private fun render(rig: RememberedConnectionFixture) {
        val saved = SavedTrustBinding(rig.scope, ConnectionAccess(), SavedTrustRunFactory { _, _ ->
            error("T1431: display controls must not start saved-trust work")
        })
        compose.setContent { BlentTheme { Column {
            ConnectionControls(rig.binding, rig.configured, saved)
        } } }
    }

    @Test fun t1431DisconnectCancelsInitialRememberedBackoff() {
        cancelBackoff(emptyList())
    }

    @Test fun t1431DisconnectCancelsCappedPreAuthenticationFailureChain() {
        cancelBackoff(listOf(2000L, 6000L, 14000L, 30000L, 60000L))
    }

    private fun cancelBackoff(failedRetries: List<Long>) {
        RememberedConnectionFixture().use { rig ->
            rig.end(rig.ready())
            for (deadline in failedRetries) {
                rig.dispatcher.advanceTo(deadline)
                rig.end(rig.workers.last())
            }
            if (failedRetries.isNotEmpty()) assertNull(rig.binding.state.value.retryPeer)
            assertNotNull(rig.dispatcher.nextTimerAt())
            val count = rig.workers.size
            render(rig)
            compose.onNodeWithText("Disconnect USB").assertIsEnabled().performClick()
            compose.runOnIdle { rig.dispatcher.advanceTo(300000) }
            assertEquals(count, rig.workers.size)
            assertNull(rig.dispatcher.nextTimerAt())
            compose.onNodeWithText("Disconnect USB").assertIsNotEnabled()
        }
    }

    @Test fun t1431UntouchedAndUnprovenEndedConnectionsDoNotOfferDisconnect() {
        RememberedConnectionFixture().use { rig ->
            render(rig)
            compose.onNodeWithText("Disconnect USB").assertIsNotEnabled()
            compose.runOnIdle { rig.end(rig.connect()) }
            assertNull(rig.dispatcher.nextTimerAt())
            compose.onNodeWithText("Disconnect USB").assertIsNotEnabled()
        }
    }
}
