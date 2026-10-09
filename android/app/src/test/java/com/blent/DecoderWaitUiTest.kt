package com.blent

import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.createComposeRule
import org.junit.Rule
import org.junit.Test
import org.junit.Assert.*
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [27, 34], qualifiers = "w1280dp-h800dp")
class DecoderWaitUiTest {
    @get:Rule val compose = createComposeRule()

    @Test fun t806_decoderWaitSurvivesPresentationReplacementAndClearsOnlyAtCompletion() {
        assertTrue(CodecLifetime.beginStartup())
        try {
            val presentation = StreamPresentation(null, null)
            assertEquals("Decoder startup pending.", presentation.decoderStartup.value)
            compose.setContent { BlentTheme { BlentMain({}, presentation = presentation) } }
            compose.onNodeWithText("Decoder startup pending.").assertIsDisplayed()
            compose.runOnIdle { CodecLifetime.finishStartup() }
            compose.onNodeWithText("Decoder startup pending.").assertDoesNotExist()
        } finally { CodecLifetime.finishStartup() }
    }
}
