package com.blent

import org.junit.Assert.*
import org.junit.Test

class CameraConsentPolicyTest {
    private class Authority : CameraAccessAuthority {
        var live = true
        var rejects = false
        var throws = false
        var updateFailure = false
        var resultFailure = false
        var onUpdate: (() -> Unit)? = null
        val choices = mutableListOf<CameraLocalConsent>()
        val results = mutableListOf<Pair<Boolean, CameraLocalConsent>>()
        override fun current(): Boolean { if (throws) error("owner unavailable"); return live }
        override fun update(context: CameraLocalConsent): Boolean {
            if (updateFailure) error("control failed")
            choices.add(context); onUpdate?.invoke(); return !rejects
        }
        override fun permissionResult(granted: Boolean, context: CameraLocalConsent): Boolean {
            if (resultFailure) error("permission control failed")
            results.add(granted to context); return !rejects
        }
    }
    private class Rig {
        var permission = true
        val prompts = CameraPermissionPrompts()
        val launched = mutableListOf<CameraPermissionPrompt>()
        val attachment = prompts.attach { launched.add(it); true }
        val policy = CameraConsentPolicy({ permission }, prompts)
        val authority = Authority()
        val original = Any()
        fun offer(pending: Boolean = false, admitted: Boolean = false, background: Boolean = true,
            owner: Any = original, authority: Authority = this.authority) =
            CameraAccessOffer(owner, CameraLens.FRONT, background, pending, admitted, authority)
    }

    @Test fun t949ExplicitLocalStartAndIndependentBackgroundConsentSurviveOnlyOriginalLifetime() {
        val rig = Rig(); val offer = rig.offer()
        rig.policy.observe(offer); assertFalse(rig.policy.start(offer)); assertTrue(rig.authority.choices.isEmpty())
        rig.policy.foreground(true); assertTrue(rig.authority.choices.isEmpty())
        assertTrue(rig.policy.start(offer)); assertTrue(rig.policy.state.value.enabled)
        assertEquals(CameraLocalConsent(true, true, true, false), rig.authority.choices.last())
        val route = rig.offer(admitted = true); rig.policy.observe(route)
        assertTrue(rig.policy.state.value.enabled)
        assertTrue(rig.policy.background(route, true)); rig.policy.foreground(false)
        assertTrue(rig.policy.state.value.enabled)
        assertEquals(CameraLocalConsent(false, true, true, true), rig.authority.choices.last())
        assertTrue(rig.policy.background(route, false)); assertFalse(rig.policy.state.value.enabled)
        assertFalse(rig.authority.choices.last().enabled)
        rig.policy.foreground(true); assertTrue(rig.policy.start(route))
        val replacement = Authority(); val next = rig.offer(owner = Any(), authority = replacement)
        rig.policy.observe(next)
        assertFalse(rig.policy.state.value.enabled); assertFalse(rig.policy.state.value.backgroundAllowed)
        assertFalse(rig.authority.choices.last().enabled); assertTrue(replacement.choices.isEmpty())
        assertFalse(rig.policy.start(route)); assertFalse(rig.policy.stop(route)); assertFalse(rig.policy.background(route, true))
        assertTrue(rig.policy.start(next)); assertTrue(rig.policy.stop(next)); rig.policy.clear()
        assertNull(rig.policy.state.value.offer)
    }

    @Test fun t949PendingGrantResamplesPermissionAndCannotGrantReplacedOfferOrActivity() {
        val rig = Rig(); rig.permission = false
        rig.policy.foreground(true)
        val offer = rig.offer(); rig.policy.observe(offer); assertTrue(rig.policy.start(offer))
        val pending = rig.offer(pending = true); rig.policy.observe(pending)
        assertEquals(1, rig.launched.size)
        rig.policy.permissionChanged(); assertEquals(1, rig.launched.size)
        assertTrue(rig.prompts.result(rig.launched.single(), true))
        assertEquals(true to CameraLocalConsent(true, false, true, false), rig.authority.results.single())
        assertFalse(rig.policy.state.value.enabled)
        assertTrue(rig.policy.start(pending)); assertEquals(2, rig.launched.size)
        val old = rig.launched.last()
        val nextAuthority = Authority(); val next = rig.offer(pending = true, owner = Any(), authority = nextAuthority)
        rig.policy.observe(next); assertFalse(rig.prompts.result(old, true)); assertTrue(nextAuthority.results.isEmpty())
        assertTrue(rig.policy.start(next)); val nextPrompt = rig.launched.last()
        rig.policy.foreground(false); rig.prompts.detach(rig.attachment)
        rig.permission = true
        assertFalse(rig.prompts.result(nextPrompt, true)); assertTrue(nextAuthority.results.isEmpty())
        assertFalse(rig.policy.state.value.enabled)
    }

    @Test fun t949PermissionLossOrMissingBackgroundConsentClosesOriginalWithoutOpeningDialogInBackground() {
        val rig = Rig(); rig.policy.foreground(true)
        val route = rig.offer(admitted = true); rig.policy.observe(route)
        assertTrue(rig.policy.start(route)); assertTrue(rig.policy.background(route, true))
        rig.policy.foreground(false); rig.permission = false; rig.policy.permissionChanged()
        assertFalse(rig.authority.choices.last().permission); assertTrue(rig.authority.choices.last().enabled)
        assertFalse(rig.policy.state.value.enabled); assertTrue(rig.launched.isEmpty())
        rig.policy.foreground(true); rig.permission = true; assertTrue(rig.policy.start(route))
        rig.policy.background(route, false); rig.policy.foreground(false)
        assertFalse(rig.policy.state.value.enabled)
        val noBackground = rig.offer(background = false); rig.policy.observe(noBackground)
        rig.policy.foreground(true); rig.policy.start(noBackground); rig.policy.background(noBackground, true)
        rig.policy.foreground(false); assertFalse(rig.policy.state.value.enabled)
    }

    @Test fun t949UnavailableLauncherAndRejectedOrDeadAuthorityCannotGrantCapture() {
        val rig = Rig(); rig.permission = false; rig.prompts.detach(rig.attachment)
        rig.policy.foreground(true); val pending = rig.offer(pending = true)
        rig.policy.observe(pending); assertTrue(rig.policy.start(pending))
        assertEquals(false to CameraLocalConsent(true, false, true, false), rig.authority.results.single())
        assertFalse(rig.policy.state.value.enabled)
        rig.authority.rejects = true; assertFalse(rig.policy.start(pending))
        rig.authority.live = false; assertFalse(rig.policy.start(pending))
        rig.authority.throws = true; assertFalse(rig.policy.start(pending))
        rig.policy.clear(); rig.policy.permissionChanged(); rig.policy.foreground(false)
        val brokenPermission = CameraConsentPolicy({ error("permission unavailable") }, CameraPermissionPrompts())
        val authority = Authority(); val offer = rig.offer(authority = authority)
        brokenPermission.observe(offer); brokenPermission.foreground(true); assertTrue(brokenPermission.start(offer))
        assertFalse(authority.choices.last().permission); brokenPermission.clear()
    }

    @Test fun t949SynchronousPublicationAndPermissionCallbackKeepExactOriginalIntent() {
        val rig = Rig(); rig.permission = false; rig.policy.foreground(true)
        val offer = rig.offer(); val pending = rig.offer(pending = true)
        rig.authority.onUpdate = { rig.authority.onUpdate = null; rig.policy.observe(pending) }
        rig.prompts.attach { rig.permission = true; rig.prompts.result(it, true) }
        rig.policy.observe(offer); assertTrue(rig.policy.start(offer))
        assertEquals(true to CameraLocalConsent(true, true, true, false), rig.authority.results.single())
        assertTrue(rig.policy.state.value.enabled); assertSame(pending, rig.policy.state.value.offer)
        rig.policy.observe(pending); assertEquals(1, rig.authority.results.size)
        rig.policy.clear(); assertFalse(rig.authority.choices.last().enabled)
    }

    @Test fun t949ReplacingPromptLauncherCannotLeavePortablePolicyWaitingOnDiscardedSlot() {
        val rig = Rig(); rig.permission = false; rig.policy.foreground(true)
        val pending = rig.offer(pending = true); rig.policy.observe(pending); rig.policy.start(pending)
        val old = rig.launched.single()
        val next = mutableListOf<CameraPermissionPrompt>()
        rig.prompts.attach { next.add(it); true }
        assertFalse(rig.prompts.result(old, true))
        rig.policy.permissionChanged()
        assertEquals("T949 discarded slot must not stall original consent", 1, next.size)
        assertNotSame(old, next.single()); rig.permission = true
        assertTrue(rig.prompts.result(next.single(), true))
        assertEquals(true to CameraLocalConsent(true, true, true, false), rig.authority.results.single())
        assertTrue(rig.policy.state.value.enabled); rig.policy.clear()
    }

    @Test fun t949RejectedThrowingAndDeniedPermissionResultsCannotKeepLocalCaptureEnabled() {
        for (mode in 0..2) {
            val rig = Rig(); rig.permission = false; rig.policy.foreground(true)
            val pending = rig.offer(pending = true); rig.policy.observe(pending); rig.policy.start(pending)
            rig.permission = true
            rig.authority.resultFailure = mode == 0; rig.authority.rejects = mode == 1
            assertTrue(rig.prompts.result(rig.launched.single(), mode != 2))
            assertFalse(rig.policy.state.value.enabled)
            rig.authority.resultFailure = false; rig.authority.rejects = false; rig.authority.updateFailure = true
            assertFalse(rig.policy.start(pending)); assertFalse(rig.policy.state.value.enabled)
            rig.policy.clear()
        }
    }

    @Test fun t949BackgroundPermissionLossWhilePreparingClosesOriginalAndNeverPrompts() {
        val rig = Rig(); val offer = rig.offer()
        rig.policy.observe(offer); rig.policy.foreground(true)
        rig.policy.background(offer, true); rig.policy.start(offer)
        rig.policy.foreground(false); assertTrue(rig.policy.state.value.enabled)
        rig.permission = false; rig.policy.permissionChanged()
        assertFalse("T949 permission loss cannot retain background intent", rig.policy.state.value.enabled)
        assertFalse(rig.authority.choices.last().enabled)
        assertFalse(rig.authority.choices.last().permission); assertTrue(rig.launched.isEmpty())
    }

    @Test fun t949InvalidOfferFlagsAndBoundedBooleanLifecyclePropertyFailClosed() {
        assertThrows(IllegalArgumentException::class.java) {
            CameraAccessOffer(Any(), CameraLens.REAR, true, true, true, Authority())
        }
        for (flags in 0..31) {
            val rig = Rig(); rig.permission = flags and 1 != 0
            val offer = rig.offer(background = flags and 2 != 0)
            rig.policy.observe(offer); rig.policy.foreground(true)
            rig.policy.background(offer, flags and 4 != 0)
            rig.policy.start(offer); rig.policy.foreground(flags and 8 != 0)
            if (flags and 16 != 0) { rig.permission = false; rig.policy.permissionChanged() }
            val view = rig.policy.state.value
            if (!view.foreground && view.enabled) {
                assertTrue("T949 background intent must retain fresh permission", rig.permission)
                assertTrue(view.backgroundAllowed); assertTrue(offer.backgroundRequested)
            }
            if (!rig.permission && !view.foreground) assertFalse(view.enabled)
            rig.policy.clear(); assertNull(rig.policy.state.value.offer); assertFalse(rig.policy.state.value.enabled)
        }
    }
}
