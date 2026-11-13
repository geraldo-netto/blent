package com.blent.connection

import androidx.compose.foundation.layout.Column
import androidx.compose.ui.test.assertIsNotEnabled
import androidx.compose.ui.test.onNodeWithText
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
class RetryDisconnectProbe {
    @get:Rule val compose = createComposeRule()
    @Test fun retryBackoffHasNoEnabledDisconnectControl() {
        RememberedConnectionFixture().use { rig ->
            ConnectionUiFixture().use { ui ->
                rig.end(rig.ready())
                assertEquals(2000L, rig.dispatcher.nextTimerAt())
                assertFalse(rig.binding.state.value.active)
                assertFalse(rig.configured.state.value.pending)
                assertTrue(rig.binding.state.value.reconnectAllowed)
                compose.setContent { BlentTheme { Column {
                    ConnectionControls(rig.binding, rig.configured, ui.saved)
                } } }
                compose.onNodeWithText("Disconnect USB").assertIsNotEnabled()
                compose.runOnIdle {
                    rig.dispatcher.advanceTo(2000)
                    assertEquals(2, rig.workers.size)
                    rig.end(rig.workers.last())
                    assertEquals(6000L, rig.dispatcher.nextTimerAt())
                    rig.configured.disconnect()
                    rig.dispatcher.advanceTo(60000)
                    assertEquals(2, rig.workers.size)
                    assertNull(rig.dispatcher.nextTimerAt())
                }
                println("Confirmed: pending retry at2000ms, Disconnect disabled; retry starts; direct disconnect cancels next6000ms retry")
            }
        }
    }
}
