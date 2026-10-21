package com.blent

import androidx.compose.ui.semantics.SemanticsActions
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.createComposeRule
import kotlinx.coroutines.*
import kotlinx.coroutines.flow.MutableStateFlow
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.annotation.Config
import org.robolectric.annotation.LooperMode

@RunWith(com.blent.ComposeFixtureRunner::class)
@Config(sdk = [27, 34], qualifiers = "w960dp-h600dp-land", instrumentedPackages = ["androidx.compose.ui.platform"])
@LooperMode(LooperMode.Mode.PAUSED)
class AudioConsentControlsTest {
    @get:Rule val compose = createComposeRule()
    private class Authority : MediaAccessAuthority {
        var live = true
        val contexts = mutableListOf<MediaLocalConsent>()
        override fun current() = live
        override fun update(context: MediaLocalConsent): Boolean { contexts.add(context); return live }
        override fun permissionResult(granted: Boolean, context: MediaLocalConsent) = granted
    }
    private class Rig(direction: Int = 2) : AutoCloseable {
        val scope = CoroutineScope(SupervisorJob() + Dispatchers.Unconfined)
        val binding = AudioBinding(AudioPreferences(), {}, { true }, { _, _, _ -> error("T985 consent is not native playback") },
            MutableStateFlow(null), scope, {}, direction, {})
        val prompts = LocalPermissionPrompts()
        var permission = true
        val requested = mutableListOf<LocalPermissionPrompt>()
        val policy = MediaConsentPolicy<AudioAccessOffer>({ permission }, prompts)
        init { prompts.attach { requested.add(it); true }; policy.foreground(true) }
        fun offer(authority: Authority, background: Boolean = false, admitted: Boolean = false, permission: Boolean = false,
            owner: Any = Any()) = AudioAccessOffer(owner, binding.direction, background, permission, admitted, authority)
        override fun close() { policy.clear(); prompts.clear(); binding.shutdown(); scope.cancel() }
    }
    private fun render(rig: Rig) {
        compose.setContent { BlentTheme { SettingsSheet(SettingsValues(), null, {}, false, {}, onSettingsEvent = {},
            cameraControls = { AudioControls(rig.binding, rig.policy) }) } }
    }
    @Test fun t1247RenderedOldStartCannotFollowARecomposedReplacement(): Unit = Rig().use { rig ->
        val firstOwner = listOf("T1247 equal metadata")
        rig.policy.observe(rig.offer(Authority(), owner = firstOwner)); render(rig)
        val old = compose.onNodeWithText("Start speakers").performScrollTo().fetchSemanticsNode()
            .config[SemanticsActions.OnClick].action!!
        val next = Authority()
        val nextOwner = listOf("T1247 equal metadata")
        assertEquals(firstOwner, nextOwner); assertNotSame(firstOwner, nextOwner)
        compose.runOnIdle { rig.policy.observe(rig.offer(next, owner = nextOwner)) }
        compose.onNodeWithText("Start speakers").performScrollTo().assertIsEnabled()
        compose.runOnIdle { old(); assertFalse("T1247 old action granted replacement", rig.policy.state.value.enabled) }
        assertTrue("T1247 old action reached replacement authority", next.contexts.isEmpty())
    }

    @Test fun t1247RenderedOldStopCannotStopARecomposedReplacement(): Unit = Rig().use { rig ->
        rig.policy.observe(rig.offer(Authority())); render(rig)
        compose.onNodeWithText("Start speakers").performScrollTo().performClick()
        val old = compose.onNodeWithText("Stop speakers").fetchSemanticsNode().config[SemanticsActions.OnClick].action!!
        compose.runOnIdle { rig.policy.observe(rig.offer(Authority())) }
        compose.onNodeWithText("Start speakers").performScrollTo().performClick()
        compose.runOnIdle { old(); assertTrue("T1247 old Stop stopped replacement", rig.policy.state.value.enabled) }
    }

    @Test fun t1247RenderedOldBackgroundToggleCannotGrantReplacement(): Unit = Rig().use { rig ->
        rig.policy.observe(rig.offer(Authority(), background = true)); render(rig)
        val old = compose.onNodeWithContentDescription("Allow this speakers request in background")
            .performScrollTo().fetchSemanticsNode().config[SemanticsActions.OnClick].action!!
        compose.runOnIdle { rig.policy.observe(rig.offer(Authority(), background = true)) }
        compose.onNodeWithContentDescription("Allow this speakers request in background").assertIsOff()
        compose.runOnIdle { old(); assertFalse("T1247 old toggle granted replacement background", rig.policy.state.value.backgroundAllowed) }
    }

    @Test fun t1247SameOwnerPublicationRetainsItsCurrentRenderedControl(): Unit = Rig().use { rig ->
        val owner = Any()
        rig.policy.observe(rig.offer(Authority(), owner = owner)); render(rig)
        val old = compose.onNodeWithText("Start speakers").performScrollTo().fetchSemanticsNode()
            .config[SemanticsActions.OnClick].action!!
        val current = Authority()
        compose.runOnIdle { rig.policy.observe(rig.offer(current, admitted = true, owner = owner)) }
        compose.onNodeWithText("Start speakers").performScrollTo().assertIsEnabled()
        assertTrue(rig.policy.state.value.offer!!.admitted)
        compose.runOnIdle { old(); assertTrue(rig.policy.state.value.enabled) }
        assertTrue(current.contexts.last().enabled)
    }

    @Test fun t985SpeakerStartStopAndWithdrawalKeepSettingsPassiveAndOriginalCallbacks(): Unit = Rig().use { rig ->
        val original = rig.offer(Authority()); rig.policy.observe(original); render(rig)
        val stale = compose.onNodeWithText("Start speakers").performScrollTo().fetchSemanticsNode().config[SemanticsActions.OnClick].action!!
        compose.onNodeWithText("Start speakers").performClick()
        compose.onNodeWithText("Audio requested; waiting for admission.").performScrollTo().assertExists()
        compose.onNodeWithText("Stop speakers").performScrollTo().performClick()
        compose.runOnIdle { assertFalse(rig.policy.state.value.enabled); assertTrue(rig.requested.isEmpty()) }
        compose.runOnIdle {
            rig.policy.observe(rig.offer(Authority(), admitted = true)); stale()
            assertFalse(rig.policy.state.value.enabled); assertFalse(rig.binding.sharing)
        }
        compose.onNodeWithText("Start speakers").performScrollTo().performClick()
        compose.onNodeWithText("Audio admitted; playback/capture follows local permission and lifecycle.").performScrollTo().assertExists()
        compose.runOnIdle { rig.policy.observe(null) }
        compose.onNodeWithText("Allow speakers").performScrollTo().assertIsNotEnabled()
    }
    @Test fun t985PersistedBackgroundPreferenceDoesNotGrantNewOfferConsent(): Unit = Rig().use { rig ->
        val offer = rig.offer(Authority(), background = true); rig.policy.observe(offer); render(rig)
        compose.onNodeWithText("Start speakers").performScrollTo().assertIsNotEnabled()
        compose.onNodeWithContentDescription("Allow background speakers").performScrollTo().performClick()
        compose.runOnIdle { assertTrue(rig.binding.preferences.background); assertFalse(rig.policy.state.value.enabled) }
        compose.onNodeWithText("Start speakers").performScrollTo().performClick()
        compose.runOnIdle { rig.policy.foreground(false); assertFalse(rig.policy.state.value.enabled) }
        compose.onNodeWithText("Start speakers").performScrollTo().assertIsNotEnabled()
        compose.runOnIdle { rig.policy.foreground(true) }
        compose.onNodeWithContentDescription("Allow this speakers request in background").performScrollTo().performClick()
        compose.onNodeWithText("Start speakers").performScrollTo().performClick()
        compose.runOnIdle { rig.policy.foreground(false); assertTrue(rig.policy.state.value.enabled) }
        compose.onNodeWithContentDescription("Allow this speakers request in background").performScrollTo().performClick()
        compose.runOnIdle { assertFalse(rig.policy.state.value.enabled) }
    }
    @Test fun t985QueuedStartRechecksBackgroundRestrictionAndNoHostRequestDisablesSwitch(): Unit = Rig().use { rig ->
        compose.runOnIdle { rig.binding.configure(AudioPreferences(background = true)) }
        val offer = rig.offer(Authority(), background = true); rig.policy.observe(offer); render(rig)
        val stale = compose.onNodeWithText("Start speakers").performScrollTo().fetchSemanticsNode().config[SemanticsActions.OnClick].action!!
        compose.runOnIdle { rig.binding.configure(AudioPreferences(background = false)); stale(); assertFalse(rig.policy.state.value.enabled) }
        compose.runOnIdle { rig.policy.observe(rig.offer(Authority())) }
        compose.onNodeWithContentDescription("Allow this speakers request in background").performScrollTo().assertIsNotEnabled()
    }
    @Test fun t985MicrophoneSdkDenialAndFreshLocalIntentCannotGrantSpeakers(): Unit = Rig(1).use { rig ->
        rig.permission = false
        val offer = rig.offer(Authority(), permission = true); rig.policy.observe(offer); render(rig)
        compose.onNodeWithText("Start microphone").performScrollTo().performClick()
        compose.onNodeWithText("Audio requested; waiting for Android permission.").performScrollTo().assertExists()
        compose.runOnIdle {
            assertEquals(1, rig.requested.size); assertTrue(rig.prompts.result(rig.requested.single(), false))
            assertFalse(rig.policy.state.value.enabled); assertFalse(rig.binding.sharing)
        }
        compose.onNodeWithText("Audio is off. Start locally to accept this request.").performScrollTo().assertExists()
    }
    @Test fun t985EmptyConsentDoesNotRenderAnyResourceOrGrantPermission(): Unit = Rig().use { rig ->
        compose.setContent { BlentTheme { AudioConsentControls(rig.binding, rig.policy, MediaConsentView()) } }
        compose.onNodeWithText("Start speakers").assertDoesNotExist()
        assertTrue(rig.requested.isEmpty())
    }
}
