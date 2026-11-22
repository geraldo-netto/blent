package com.blent

import androidx.compose.ui.semantics.SemanticsActions
import androidx.compose.ui.semantics.SemanticsProperties
import androidx.compose.ui.semantics.getOrNull
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.createComposeRule
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RuntimeEnvironment
import org.robolectric.annotation.Config
import org.robolectric.annotation.LooperMode

@RunWith(com.blent.ComposeFixtureRunner::class)
@Config(sdk = [27, 34], qualifiers = "w960dp-h600dp-land")
@LooperMode(LooperMode.Mode.PAUSED)
class SettingsControlContractTest {
    @get:Rule val compose = createComposeRule()

    @Test fun t497_displayChoicesExcludeNonfiniteNegativeAndDuplicatePanelModes() {
        val events = mutableListOf<SettingsEvent>()
        val rates = listOf(Float.NaN, Float.POSITIVE_INFINITY, Float.NEGATIVE_INFINITY, -1f, 0f, 60f, 120f, 60f)
        compose.setContent { BlentTheme {
            androidx.compose.foundation.layout.Column { DisplayControls(SettingsValues(), rates) { events.add(it) } }
        } }
        compose.onAllNodesWithText("60 Hz").assertCountEquals(1)
        compose.onAllNodesWithText("System default").assertCountEquals(1)
        compose.onNodeWithText("120 Hz").performClick()
        compose.onNodeWithText("NaN Hz").assertDoesNotExist()
        compose.onNodeWithText("-1 Hz").assertDoesNotExist()
        compose.runOnIdle { assertEquals(listOf(SettingsEvent.RefreshRate(120f)), events) }
    }

    @Test fun t497_optionalMainCallbacksPermitDismissalAndSettings() {
        val visible = androidx.compose.runtime.mutableStateOf(true)
        compose.setContent { BlentTheme {
            if (visible.value) BlentMain(onSurfaceReady = {}, showThanks = true)
        } }
        compose.onNodeWithText("Dismiss").performClick()
        compose.onNodeWithText("⚙").performClick()
        compose.onNodeWithContentDescription("Battery saver").performScrollTo().performClick()
        compose.onNodeWithText("Done").performScrollTo().performClick()
        compose.onNodeWithText("Settings").assertDoesNotExist()
        compose.runOnIdle { visible.value = false }
        compose.waitForIdle()
        compose.onNodeWithText("Blent is working.").assertDoesNotExist()
    }

    @Test fun t497_settingsSheetAllowsAnAbsentSettingsObserver() {
        var dismissals = 0
        compose.setContent { BlentTheme {
            SettingsSheet(SettingsValues(), null, {}, false, { dismissals++ })
        } }
        compose.onNodeWithContentDescription("Battery saver").performScrollTo().performClick()
        compose.onNodeWithText("Done").performScrollTo().performClick()
        compose.runOnIdle { assertEquals(1, dismissals) }
    }

    @Test fun t497_settingsControlsDeliverTheirUserSelectedValues() {
        val prefs = Prefs(RuntimeEnvironment.getApplication())
        val session = SessionCoordinator(prefs, { it() }, null, null)
        val events = mutableListOf<SettingsEvent>()
        var dismissals = 0
        var updates = 0
        compose.setContent { BlentTheme {
            SettingsSheet(session.settings, "v-test", { updates++ }, false, { dismissals++ },
                onSettingsEvent = { events.add(it); session.handle(it) })
        } }
        compose.onNodeWithText("Update available: v-test").performScrollTo().performClick()
        toggleBeside("Rotate automatically").performScrollTo().performClick()
        compose.onNodeWithText("Camera up").performScrollTo().performClick().assertIsSelected()
        compose.onNodeWithText("Camera down").performClick().assertIsSelected()
        toggleBeside("Rotate automatically").performScrollTo().performClick()
        compose.onNodeWithText("App & diagnostics").performScrollTo().performClick()
        compose.onNodeWithContentDescription("Show stats overlay").performScrollTo().performClick().assertIsOn()
        compose.onNodeWithContentDescription("Check for newer releases").performScrollTo().performClick().assertIsOff()
        compose.runOnIdle {
            assertEquals(1, updates)
            assertEquals(Prefs.ORIENTATION_AUTO, prefs.orientation)
            assertTrue(prefs.showStats)
            assertFalse(prefs.checkUpdates)
            assertEquals(0, dismissals)
        }
        compose.onNodeWithText("30 fps").performScrollTo().performClick().assertIsSelected()
        compose.onNodeWithText("Advanced video settings").performScrollTo().performClick()
        val bitrate = SemanticsMatcher("stream bitrate range") {
            it.config.getOrNull(SemanticsProperties.ProgressBarRangeInfo)?.range ==
                (Prefs.MIN_BITRATE_KBPS / 1000f)..(Prefs.MAX_BITRATE_KBPS / 1000f)
        }
        compose.onNode(bitrate).performScrollTo().performSemanticsAction(SemanticsActions.SetProgress) { it(60f) }
        compose.onNodeWithText("Apply video changes").performScrollTo().performClick()
        compose.runOnIdle {
            assertTrue(events.contains(SettingsEvent.Stream(60000, 30)))
            assertEquals(60000, prefs.streamOverrides.bitrateKbps)
            assertEquals(30, prefs.streamOverrides.fps)
            assertEquals(1, dismissals)
        }
        toggleBeside("Graphics tablet").performScrollTo().performClick()
        compose.runOnIdle {
            assertEquals(SettingsEvent.Mode(true), events.last())
            assertEquals(2, dismissals)
        }
    }

    @Test fun t1570BitrateGridContainsWholeMegabitsAndReturnsToDefault() {
        val events = mutableListOf<SettingsEvent>()
        compose.setContent { BlentTheme {
            SettingsSheet(SettingsValues(), null, {}, false, {}, onSettingsEvent = events::add)
        } }
        compose.onNodeWithText("Advanced video settings").performScrollTo().performClick()
        val slider = bitrateSlider().performScrollTo()
        assertEquals("T1570 whole-Mbps grid", 58,
            slider.fetchSemanticsNode().config[SemanticsProperties.ProgressBarRangeInfo].steps)
        for (value in listOf(1, 2, 19, 20, 21, 59, 60, 20)) {
            slider.performSemanticsAction(SemanticsActions.SetProgress) { it(value.toFloat()) }
            compose.onNodeWithText("Bitrate: $value Mbps").assertExists()
        }
        compose.onNodeWithText("Apply video changes").performScrollTo().performClick()
        compose.runOnIdle { assertTrue(events.none { it is SettingsEvent.Stream }) }
    }

    @Test fun t1570ExistingFractionalBitrateLabelMatchesSubmittedKilobits() {
        val events = mutableListOf<SettingsEvent>()
        compose.setContent { BlentTheme {
            SettingsSheet(SettingsValues(bitrateKbps = 25005), null, {}, false, {}, onSettingsEvent = events::add)
        } }
        compose.onNodeWithText("Advanced video settings").performScrollTo().performClick()
        compose.onNodeWithText("Bitrate: 25.005 Mbps").performScrollTo().assertIsDisplayed()
        compose.onNodeWithText("Apply video changes").performScrollTo().performClick()
        compose.runOnIdle { assertTrue(events.none { it is SettingsEvent.Stream }) }
    }

    @Test fun t1694ApplyingAnUntouchedDraftSendsNoStreamOverride() {
        val events = mutableListOf<SettingsEvent>()
        compose.setContent { BlentTheme {
            SettingsSheet(SettingsValues(bitrateKbps = 25005, fps = 60), null, {}, false, {},
                onSettingsEvent = events::add)
        } }
        compose.onNodeWithText("Apply video changes").performScrollTo().performClick()
        compose.runOnIdle { assertTrue("T1694 unchanged settings became an override", events.none { it is SettingsEvent.Stream }) }
    }

    @Test fun t1694FpsOnlyEditDoesNotSubmitUnchangedBitrate() {
        val events = mutableListOf<SettingsEvent>()
        compose.setContent { BlentTheme {
            SettingsSheet(SettingsValues(bitrateKbps = 25005, fps = 60), null, {}, false, {},
                onSettingsEvent = events::add)
        } }
        compose.onNodeWithText("30 fps").performScrollTo().performClick()
        compose.onNodeWithText("Apply video changes").performScrollTo().performClick()
        compose.runOnIdle {
            val event = events.filterIsInstance<SettingsEvent.Stream>().single()
            assertEquals(30, event.fps)
            assertNull("T1694 unedited bitrate was submitted", event.bitrate)
        }
    }

    @Test fun t1694BitrateOnlyEditDoesNotSubmitUnchangedFps() {
        val events = mutableListOf<SettingsEvent>()
        compose.setContent { BlentTheme {
            SettingsSheet(SettingsValues(bitrateKbps = 20000, fps = 90), null, {}, false, {},
                onSettingsEvent = events::add)
        } }
        compose.onNodeWithText("Advanced video settings").performScrollTo().performClick()
        bitrateSlider().performScrollTo().performSemanticsAction(SemanticsActions.SetProgress) { it(25f) }
        compose.onNodeWithText("Apply video changes").performScrollTo().performClick()
        compose.runOnIdle {
            val event = events.filterIsInstance<SettingsEvent.Stream>().single()
            assertEquals(25000, event.bitrate)
            assertNull("T1694 unedited FPS was submitted", event.fps)
        }
    }

    @Test fun t1694StoredStreamOverridesOfferUseHostSettings() {
        val prefs = Prefs(RuntimeEnvironment.getApplication()).apply { checkUpdates = false }
        val session = SessionCoordinator(prefs, { it() }, null, null)
        try {
            session.handle(SettingsEvent.Stream(25000, 90))
            compose.setContent { BlentTheme {
                SettingsSheet(session.settings, null, {}, false, {}, onSettingsEvent = session::handle)
            } }
            compose.onNodeWithText("Use host settings").performScrollTo().assertIsDisplayed().performClick()
            compose.runOnIdle { assertTrue(prefs.streamOverrides.empty) }
            compose.onNodeWithText("Use host settings").assertDoesNotExist()
        } finally { session.stop() }
    }

    @Test fun t1694HostRefreshPreservesDraftAndRefreshesUntouchedFields() {
        val settings = androidx.compose.runtime.mutableStateOf(SettingsValues())
        val events = mutableListOf<SettingsEvent>()
        compose.setContent { BlentTheme {
            SettingsSheet(settings.value, null, {}, false, {}, onSettingsEvent = events::add)
        } }
        compose.onNodeWithText("30 fps").performScrollTo().performClick()
        compose.runOnIdle { settings.value = settings.value.copy(bitrateKbps = 25005, fps = 90) }
        compose.onNodeWithText("30 fps").assertIsSelected()
        compose.onNodeWithText("Advanced video settings").performScrollTo().performClick()
        compose.onNodeWithText("Bitrate: 25.005 Mbps").performScrollTo().assertIsDisplayed()
        compose.onNodeWithText("Apply video changes").performScrollTo().performClick()
        compose.runOnIdle { assertEquals(listOf(SettingsEvent.Stream(null, 30)), events) }
    }

    private fun bitrateSlider() = compose.onNode(SemanticsMatcher("stream bitrate range") {
        it.config.getOrNull(SemanticsProperties.ProgressBarRangeInfo)?.range == 1f..60f
    })

    private fun toggleBeside(text: String) = compose.onNodeWithContentDescription(text)
}
