package com.blent

import kotlinx.coroutines.*
import kotlinx.coroutines.flow.*
import org.junit.Assert.*
import org.junit.Test

/** T982: independent audio consent uses the same original-owner policy as camera. */
class MediaConsentAudioTest {
    private class Authority : MediaAccessAuthority {
        var live = true
        val choices = mutableListOf<MediaLocalConsent>()
        val results = mutableListOf<Pair<Boolean, MediaLocalConsent>>()
        override fun current() = live
        override fun update(context: MediaLocalConsent) = choices.add(context)
        override fun permissionResult(granted: Boolean, context: MediaLocalConsent) = results.add(granted to context)
    }
    private class Rig(val direction: Int) {
        var permission = true
        val prompts = LocalPermissionPrompts()
        val launched = mutableListOf<LocalPermissionPrompt>()
        val attachment = prompts.attach { launched.add(it); true }
        val authority = Authority()
        val original = Any()
        val policy = MediaConsentPolicy<AudioAccessOffer>({ permission }, prompts)
        fun offer(owner: Any = original, authority: Authority = this.authority,
            pending: Boolean = false, admitted: Boolean = false, background: Boolean = true) =
            AudioAccessOffer(owner, direction, background, pending, admitted, authority)
    }

    @Test fun t982EachDirectionRequiresLocalStartAndBackgroundConsentForOriginalOwner() {
        for (direction in 1..2) {
            val rig = Rig(direction); val offer = rig.offer()
            rig.policy.observe(offer)
            assertFalse(rig.policy.start(offer)); assertTrue(rig.authority.choices.isEmpty())
            rig.policy.foreground(true); assertTrue(rig.authority.choices.isEmpty())
            assertTrue(rig.policy.start(offer)); assertEquals(direction, rig.policy.state.value.offer?.direction)
            assertEquals(MediaLocalConsent(true, true, true, false), rig.authority.choices.last())
            val admitted = rig.offer(admitted = true); rig.policy.observe(admitted)
            assertTrue(rig.policy.state.value.enabled)
            assertTrue(rig.policy.background(admitted, true)); rig.policy.foreground(false)
            assertTrue(rig.policy.state.value.enabled)
            assertEquals(MediaLocalConsent(false, true, true, true), rig.authority.choices.last())
            assertTrue(rig.policy.background(admitted, false)); assertFalse(rig.policy.state.value.enabled)
            assertFalse(rig.authority.choices.last().enabled)
            rig.policy.clear(); assertNull(rig.policy.state.value.offer)
        }
    }

    @Test fun t982EqualMetadataCannotCarryIntentOrStaleCallbacksAcrossOwners() {
        for (direction in 1..2) {
            val rig = Rig(direction); val old = rig.offer()
            rig.policy.observe(old); rig.policy.foreground(true); rig.policy.start(old)
            rig.policy.background(old, true)
            val replacement = Authority(); val next = rig.offer(owner = Any(), authority = replacement)
            rig.policy.observe(next)
            assertSame(next, rig.policy.state.value.offer)
            assertFalse(rig.policy.state.value.enabled); assertFalse(rig.policy.state.value.backgroundAllowed)
            assertFalse(rig.authority.choices.last().enabled); assertTrue(replacement.choices.isEmpty())
            assertFalse(rig.policy.start(old)); assertFalse(rig.policy.stop(old))
            assertFalse(rig.policy.background(old, true)); assertTrue(replacement.choices.isEmpty())
            assertTrue(rig.policy.start(next)); assertTrue(rig.policy.stop(next))
            assertFalse(rig.policy.state.value.enabled)
            rig.policy.clear()
        }
    }

    @Test fun t982MicrophoneResultResamplesPermissionAndCannotGrantReplacementOrHiddenOwner() {
        val rig = Rig(1); rig.permission = false; rig.policy.foreground(true)
        val pending = rig.offer(pending = true); rig.policy.observe(pending); rig.policy.start(pending)
        assertEquals(1, rig.launched.size); rig.policy.permissionChanged(); assertEquals(1, rig.launched.size)
        assertTrue(rig.prompts.result(rig.launched.single(), true))
        assertEquals(true to MediaLocalConsent(true, false, true, false), rig.authority.results.single())
        assertFalse(rig.policy.state.value.enabled)
        rig.policy.start(pending); val old = rig.launched.last()
        val replacement = Authority(); val next = rig.offer(owner = Any(), authority = replacement, pending = true)
        rig.policy.observe(next)
        assertFalse(rig.prompts.result(old, true)); assertTrue(replacement.results.isEmpty())
        rig.policy.start(next); val newer = rig.launched.last()
        rig.policy.foreground(false); rig.permission = true
        assertFalse(rig.prompts.result(newer, true)); assertTrue(replacement.results.isEmpty())
        assertFalse(rig.policy.state.value.enabled); rig.policy.clear()
    }

    @Test fun t982MicrophoneDeniedOrLostPermissionNeverAffectsSpeakerAndPlaybackNeverPrompts() {
        val microphone = Rig(1); val speaker = Rig(2)
        microphone.permission = false
        val pending = microphone.offer(pending = true); val playing = speaker.offer(admitted = true)
        microphone.policy.observe(pending); speaker.policy.observe(playing)
        microphone.policy.foreground(true); speaker.policy.foreground(true)
        assertTrue(speaker.policy.start(playing)); assertTrue(speaker.launched.isEmpty())
        microphone.policy.start(pending)
        assertTrue(microphone.prompts.result(microphone.launched.single(), false))
        assertFalse(microphone.policy.state.value.enabled); assertTrue(speaker.policy.state.value.enabled)
        assertEquals(false to MediaLocalConsent(true, false, true, false), microphone.authority.results.single())
        microphone.policy.clear(); assertTrue(speaker.policy.state.value.enabled)
        speaker.authority.live = false; assertFalse(speaker.policy.start(playing))
        speaker.policy.clear(); assertNull(speaker.policy.state.value.offer)
    }

    @Test fun t982UnavailableOrReplacedSdkLauncherCannotRelabelAnOutstandingMicrophonePrompt() {
        val rig = Rig(1); rig.permission = false; rig.policy.foreground(true)
        val pending = rig.offer(pending = true); rig.policy.observe(pending); rig.policy.start(pending)
        val previous = rig.launched.single()
        val next = mutableListOf<LocalPermissionPrompt>()
        rig.prompts.attach { next.add(it); true }
        assertFalse(rig.prompts.result(previous, true)); rig.policy.permissionChanged()
        assertEquals(1, next.size); assertNotSame(previous, next.single())
        rig.permission = true; assertTrue(rig.prompts.result(next.single(), true))
        assertEquals(true to MediaLocalConsent(true, true, true, false), rig.authority.results.single())
        assertTrue(rig.policy.state.value.enabled)
        rig.policy.clear()
        val refused = Rig(1); refused.permission = false; refused.prompts.detach(refused.attachment)
        refused.policy.foreground(true); val offer = refused.offer(pending = true)
        refused.policy.observe(offer); refused.policy.start(offer)
        assertEquals(false to MediaLocalConsent(true, false, true, false), refused.authority.results.single())
        assertFalse(refused.policy.state.value.enabled); refused.policy.clear()
    }

    @Test fun t982AllDirectionTagsAndOfferFlagsRejectInvalidMetadata() {
        for (direction in (0..255) + listOf(-1, Int.MIN_VALUE, Int.MAX_VALUE)) {
            for (flags in 0..7) invalidMetadata(direction, flags)
        }
    }
    private fun invalidMetadata(direction: Int, flags: Int) {
        val required = flags and 1 != 0; val admitted = flags and 2 != 0
        val valid = direction in 1..2 && !(required && admitted) && (direction == 1 || !required)
        val result = runCatching { AudioAccessOffer(Any(), direction, flags and 4 != 0, required, admitted, Authority()) }
        assertEquals("T982 direction=$direction flags=$flags", valid, result.isSuccess)
        result.getOrNull()?.let { assertEquals(direction, it.direction); assertEquals(required, it.permissionRequired) }
    }

    @Test fun t982BoundedLocalLifecycleFlagsRetainOnlyExplicitCurrentBackgroundIntent() {
        for (direction in 1..2) for (flags in 0..31) backgroundProperty(direction, flags)
    }
    private fun backgroundProperty(direction: Int, flags: Int) {
        val rig = Rig(direction); rig.permission = flags and 1 != 0
        val offered = rig.offer(background = flags and 2 != 0)
        rig.policy.observe(offered); rig.policy.foreground(true)
        rig.policy.background(offered, flags and 4 != 0); rig.policy.start(offered)
        rig.policy.foreground(flags and 8 != 0)
        if (flags and 16 != 0) { rig.permission = false; rig.policy.permissionChanged() }
        val view = rig.policy.state.value
        if (!view.foreground && view.enabled) {
            assertTrue(rig.permission); assertTrue(view.backgroundAllowed); assertTrue(offered.backgroundRequested)
        }
        if (!rig.permission && !view.foreground) assertFalse(view.enabled)
        rig.policy.clear(); assertNull(rig.policy.state.value.offer); assertFalse(rig.policy.state.value.enabled)
    }

    @Test fun t982AudioSourceFailureAndRetirementRevokeOnlyTheirOriginalPolicy() = runBlocking {
        val sibling = Rig(2); val other = sibling.offer()
        sibling.policy.observe(other); sibling.policy.foreground(true); sibling.policy.start(other)
        for (failed in listOf(false, true)) {
            val rig = Rig(1); val offer = rig.offer()
            val signal = MutableSharedFlow<Unit>()
            val scope = CoroutineScope(SupervisorJob() + Dispatchers.Unconfined)
            val source = flow {
                emit(offer); rig.policy.foreground(true); rig.policy.start(offer)
                signal.first(); if (failed) error("original audio source failed")
            }
            val observation = MediaConsentObservation<AudioAccessOffer>(source, rig.policy, scope)
            assertSame(offer, rig.policy.state.value.offer); signal.emit(Unit)
            assertNull(rig.policy.state.value.offer); assertFalse(rig.authority.choices.last().enabled)
            assertTrue(sibling.policy.state.value.enabled)
            observation.retire(); scope.cancel()
        }
        sibling.policy.clear()
    }

    @Test fun t982AudioCancellationAndLatePublicationCannotReviveWithdrawnConsent() {
        val rig = Rig(2); val source = MutableStateFlow<AudioAccessOffer?>(null)
        val scope = CoroutineScope(SupervisorJob() + Dispatchers.Unconfined)
        val observation = MediaConsentObservation<AudioAccessOffer>(source, rig.policy, scope)
        val old = rig.offer(); source.value = old
        rig.policy.foreground(true); rig.policy.start(old)
        scope.cancel(); assertNull(rig.policy.state.value.offer)
        source.value = rig.offer(owner = Any()); assertNull(rig.policy.state.value.offer)
        assertFalse(rig.policy.start(old)); observation.retire(); observation.retire()
    }
}
