package com.blent

import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.luminance
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.semantics.SemanticsActions
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.createComposeRule
import androidx.compose.ui.text.TextLayoutResult
import androidx.compose.ui.unit.Density
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.annotation.Config
import org.robolectric.annotation.LooperMode

@RunWith(com.blent.ComposeFixtureRunner::class)
@Config(sdk = [27, 34], qualifiers = "w960dp-h360dp-land")
@LooperMode(LooperMode.Mode.PAUSED)
class SettingsContrastTest {
    @get:Rule val compose = createComposeRule()

    @Test fun t1784HelpTextIsReadableAtLargeFontAndLandscape() {
        var surface = Color.Unspecified
        compose.setContent { BlentTheme {
            surface = MaterialTheme.colorScheme.surface
            CompositionLocalProvider(LocalDensity provides Density(LocalDensity.current.density, 2f)) {
                SettingsSheet(SettingsValues(), "fixture", {}, false, {})
            }
        } }
        assertReadable("Follow the tilt sensor", surface)
        assertReadable("Draw on the computer", surface)
        assertReadable("Reduce background work", surface)
        assertReadable("Tap to open the release page", surface)
        compose.onNodeWithText("Advanced video settings").performScrollTo().performClick()
        assertReadable("This setting depends", surface)
        assertReadable("Apply video changes sends only edited fields", surface)
        compose.onNodeWithText("Apply video changes").performScrollTo().assertIsDisplayed()
    }

    private fun assertReadable(prefix: String, surface: Color) {
        val results = mutableListOf<TextLayoutResult>()
        compose.onNodeWithText(prefix, substring = true).performScrollTo().assertIsDisplayed()
            .performSemanticsAction(SemanticsActions.GetTextLayoutResult) { it(results) }
        val style = results.single().layoutInput.style
        assertTrue("T1784 help type too small: $prefix", style.fontSize.value >= 12f)
        val foreground = style.color.luminance()
        val background = surface.luminance()
        val contrast = (maxOf(foreground, background) + 0.05f) / (minOf(foreground, background) + 0.05f)
        assertTrue("T1784 help contrast $contrast: $prefix", contrast >= 4.5f)
    }
}
