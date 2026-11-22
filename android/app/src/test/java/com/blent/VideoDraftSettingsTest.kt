package com.blent

import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.createComposeRule
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.annotation.Config
import org.robolectric.annotation.LooperMode

@RunWith(com.blent.ComposeFixtureRunner::class)
@Config(sdk = [27, 34], qualifiers = "w960dp-h600dp-land")
@LooperMode(LooperMode.Mode.PAUSED)
class VideoDraftSettingsTest {
    @get:Rule val compose = createComposeRule()

    @Test fun t1785ApplyIsNamedAndAdjacentToVideoDraft() {
        val events = mutableListOf<SettingsEvent>()
        compose.setContent { BlentTheme {
            SettingsSheet(SettingsValues(), null, {}, false, {}, onSettingsEvent = events::add)
        } }
        compose.onNodeWithText("30 fps").performScrollTo().performClick()
        compose.onNodeWithText("Apply video changes").performScrollTo().assertIsDisplayed()
        val apply = compose.onNodeWithText("Apply video changes").fetchSemanticsNode().positionInRoot
        val app = compose.onNodeWithText("App").fetchSemanticsNode().positionInRoot
        assertTrue("T1785 Apply belongs before unrelated app controls", apply.y < app.y)
        compose.onNodeWithText("Video changes not applied").assertExists()
        compose.onNodeWithText("Apply video changes").performClick()
        compose.runOnIdle { assertEquals(listOf(SettingsEvent.Stream(null, 30)), events) }
    }
    @Test fun t1785DoneRequiresDiscardAndKeepsImmediateSwitches() {
        val events = mutableListOf<SettingsEvent>()
        var dismissals = 0
        compose.setContent { BlentTheme {
            SettingsSheet(SettingsValues(), null, {}, false, { dismissals++ }, onSettingsEvent = events::add)
        } }
        compose.onNodeWithText("30 fps").performScrollTo().performClick()
        compose.onNodeWithContentDescription("Battery saver").performScrollTo().performClick()
        compose.onNodeWithText("Done").performScrollTo().performClick()
        compose.onNodeWithText("Discard video changes?").assertIsDisplayed()
        compose.onNodeWithText("Keep editing").performClick()
        compose.onNodeWithText("30 fps").performScrollTo().assertIsSelected()
        compose.runOnIdle {
            assertEquals(0, dismissals)
            assertTrue(events.single() is SettingsEvent.BatterySaver)
        }
        compose.onNodeWithText("Done").performScrollTo().performClick()
        compose.onNodeWithText("Discard video changes").performClick()
        compose.runOnIdle {
            assertEquals(1, dismissals)
            assertTrue(events.single() is SettingsEvent.BatterySaver)
        }
        compose.onNodeWithText("No pending video edits").performScrollTo().assertIsDisplayed()
    }

    @Test fun t1785DismissGestureCannotSilentlyDropVideoEdits() {
        var dismissals = 0
        compose.setContent { BlentTheme {
            SettingsSheet(SettingsValues(), null, {}, false, { dismissals++ })
        } }
        compose.onNodeWithText("30 fps").performScrollTo().performClick()
        compose.onNode(SemanticsMatcher.keyIsDefined(androidx.compose.ui.semantics.SemanticsActions.Dismiss))
            .performSemanticsAction(androidx.compose.ui.semantics.SemanticsActions.Dismiss) { it() }
        compose.onNodeWithText("Discard video changes?").assertIsDisplayed()
        compose.onNodeWithText("Keep editing").performClick()
        compose.onNodeWithText("30 fps").performScrollTo().assertIsSelected()
        compose.runOnIdle { assertEquals(0, dismissals) }
    }

    @Test fun t1785UnchangedVideoActionIsDisabledAndDoneCloses() {
        var dismissals = 0
        val events = mutableListOf<SettingsEvent>()
        compose.setContent { BlentTheme {
            SettingsSheet(SettingsValues(), null, {}, false, { dismissals++ }, onSettingsEvent = events::add)
        } }
        compose.onNodeWithText("Apply video changes").performScrollTo().assertIsNotEnabled()
        compose.onNodeWithText("Done").performScrollTo().performClick()
        compose.runOnIdle { assertEquals(1, dismissals); assertTrue(events.isEmpty()) }
    }

}
