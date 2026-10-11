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
import org.robolectric.RobolectricTestRunner
import org.robolectric.RuntimeEnvironment
import org.robolectric.annotation.Config
import org.robolectric.annotation.LooperMode

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [27, 34], qualifiers = "w960dp-h600dp-land",
    instrumentedPackages = ["androidx.compose.ui.platform"])
@LooperMode(LooperMode.Mode.PAUSED)
class CameraConsentControlsTest {
    @get:Rule val compose = createComposeRule()

    private class Authority : CameraAccessAuthority {
        var live = true
        val contexts = mutableListOf<CameraLocalConsent>()
        override fun current() = live
        override fun update(context: CameraLocalConsent): Boolean { contexts.add(context); return live }
        override fun permissionResult(granted: Boolean, context: CameraLocalConsent): Boolean {
            contexts.add(context); return live && granted
        }
    }

    private class Rig : AutoCloseable {
        val scope = CoroutineScope(SupervisorJob() + Dispatchers.Unconfined)
        val binding = CameraBinding(RuntimeEnvironment.getApplication(), { 0 }, {}, { true },
            { _, _, _, _ -> error("T973 local consent must not start legacy capture") },
            MutableStateFlow(null), scope)
        val prompts = CameraPermissionPrompts()
        val requested = mutableListOf<CameraPermissionPrompt>()
        var permission = false
        val policy = CameraConsentPolicy({ permission }, prompts)
        init { prompts.attach { requested.add(it); true }; policy.foreground(true) }
        fun offer(authority: Authority, permissionRequired: Boolean = true, admitted: Boolean = false,
            background: Boolean = true, lens: CameraLens = CameraLens.FRONT) =
            CameraAccessOffer(Any(), lens, background, permissionRequired, admitted, authority)
        override fun close() { policy.clear(); prompts.clear(); binding.shutdown(); scope.cancel() }
    }

    @Test fun t973UsbOfferHasLocalStartBeforeAdmissionAndDeniedPermissionCannotEnableCapture() {
        Rig().use { rig ->
            val authority = Authority(); val offer = rig.offer(authority)
            rig.policy.observe(offer)
            compose.setContent { BlentTheme { CameraControls(rig.binding, rig.policy) } }
            compose.onNodeWithText("Start camera").assertIsEnabled().performClick()
            compose.runOnIdle {
                assertTrue(rig.policy.state.value.enabled)
                assertEquals(1, rig.requested.size)
                assertNull(rig.binding.state.value.endpoint)
                assertNull(rig.binding.state.value.selected)
                assertTrue(rig.prompts.result(rig.requested.single(), false))
                assertFalse(rig.policy.state.value.enabled)
            }
            compose.onNodeWithText("Start camera").assertIsEnabled()
            compose.onNodeWithText("Stop camera").assertIsNotEnabled()
        }
    }

    @Test fun t973StaleRenderedActionCannotConsentToReplacementAndWithdrawalRestoresLegacyControls() {
        Rig().use { rig ->
            val first = Authority(); val original = rig.offer(first)
            rig.policy.observe(original)
            compose.setContent { BlentTheme { CameraControls(rig.binding, rig.policy) } }
            val click = compose.onNodeWithText("Start camera").fetchSemanticsNode()
                .config[SemanticsActions.OnClick].action!!
            compose.runOnIdle {
                rig.policy.observe(rig.offer(Authority(), lens = CameraLens.REAR))
                click()
                assertFalse(rig.policy.state.value.enabled)
                assertTrue(first.contexts.isEmpty())
                assertTrue(rig.requested.isEmpty())
            }
            compose.onNodeWithText("USB camera request: Rear.").assertExists()
            compose.runOnIdle { rig.policy.observe(null) }
            compose.onNodeWithText("Start camera").assertIsNotEnabled()
        }
    }

    @Test fun t973LocalBackgroundStopAndAdmissionControlsStayIndependentOfDisplay() {
        Rig().use { rig ->
            rig.permission = true
            val authority = Authority(); val offer = rig.offer(authority, false, true)
            rig.policy.observe(offer)
            compose.setContent { BlentTheme { CameraControls(rig.binding, rig.policy) } }
            compose.onNodeWithText("USB camera request: Front.").assertExists()
            compose.onNodeWithContentDescription("Allow camera in background").performClick()
            compose.onNodeWithText("Start camera").performClick()
            compose.runOnIdle {
                assertTrue(rig.policy.state.value.backgroundAllowed)
                rig.policy.foreground(false)
                assertTrue(rig.policy.state.value.enabled)
            }
            compose.onNodeWithText("Stop camera").performClick()
            compose.runOnIdle { assertFalse(authority.contexts.last().enabled); assertNull(rig.binding.state.value.selected) }
            compose.onNodeWithText("Start camera").assertIsNotEnabled()
            compose.runOnIdle { rig.policy.foreground(true) }
            compose.onNodeWithText("Start camera").performClick()
            compose.onNodeWithText("Camera admitted; capture follows local permission and lifecycle.").assertExists()
            compose.onNodeWithContentDescription("Allow camera in background").performClick()
            compose.runOnIdle { rig.policy.foreground(false); assertFalse(rig.policy.state.value.enabled) }
        }
    }

    @Test fun t973NoHostBackgroundRequestCannotEnableBackgroundAndWorkerLossRevokesOffer() {
        Rig().use { rig ->
            rig.permission = true
            val authority = Authority(); val offer = rig.offer(authority, false, false, false)
            rig.policy.observe(offer)
            compose.setContent { BlentTheme { CameraControls(rig.binding, rig.policy) } }
            compose.onNodeWithContentDescription("Allow camera in background").assertIsNotEnabled()
            compose.onNodeWithText("Start camera").performClick()
            compose.onNodeWithText("Camera requested; waiting for admission.").assertExists()
            compose.runOnIdle {
                authority.live = false; rig.policy.observe(null)
                assertFalse(rig.policy.state.value.enabled)
            }
            compose.onNodeWithText("Start camera").assertIsNotEnabled()
            compose.onNodeWithText("USB camera request: Front.").assertDoesNotExist()
        }
    }
}
