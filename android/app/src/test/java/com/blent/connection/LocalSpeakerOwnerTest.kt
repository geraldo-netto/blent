package com.blent.connection

import com.blent.MediaLocalConsent
import org.junit.Assert.*
import org.junit.Test

private class SpeakerRig(val background: Boolean = false) : ConnectionResourceAuthority {
    val endpoints = ResourceEndpoints()
    val display = LocalPumpRig().lease
    val lease = display.copy(key = display.key.copy(resource = ConnectionManagement.Resource.SPEAKERS))
    val credential = ConnectionManagement.Credential(ByteArray(32) { 42 })
    val profile = ConnectionManagement.Audio(2, 40, background)
    val natives = ArrayList<PromptEndpoint>()
    var current = true
    var deadline = 1000L
    val local = ConnectionLocalResources(endpoints, 4096, ConnectionRunLimits(10, 5), {
        PromptEndpoint(23000 + natives.size).also { natives.add(it) }
    }, audioAuthority = this)
    override fun owns(lease: ConnectionResourceLease, now: Long): Boolean =
        current && lease == this.lease && (now < deadline || endpoints.current(lease))
    fun offer() { endpoints.events.add(ConnectionSessionEvent.Offered(lease, credential, profile)); local.advance(0) }
    fun start() { offer(); assertTrue(local.chooseAudio(lease, MediaLocalConsent(true, true, true, false), 0)); local.advance(0) }
    fun admit() { endpoints.state(lease, ConnectionResourceState.ADMITTED); local.advance(1) }
    fun close(now: Long = 2) { endpoints.state(lease, ConnectionResourceState.CLOSING); local.advance(now) }
    fun finish(now: Long) { natives.forEach { it.finish() }; local.advance(now) }
}

class LocalSpeakerOwnerTest {
    private val speakers = ConnectionManagement.Resource.SPEAKERS
    private val microphone = ConnectionManagement.Resource.MICROPHONE
    @Test fun t984ExplicitSpeakerStartPreparesOnceAndAdmissionPublishesOriginalProfileOnly() {
        val rig = SpeakerRig()
        assertNull(rig.local.audioOffered(speakers)); assertNull(rig.local.audio(speakers))
        assertFalse(rig.local.chooseAudio(rig.lease, MediaLocalConsent(true, true, true, false), 0))
        assertFalse(rig.local.audioConsumerRetired(rig.lease, 0))
        rig.offer(); val offered = checkNotNull(rig.local.audioOffered(speakers))
        assertSame(offered, rig.local.audioOffered(speakers)); assertEquals(rig.profile, offered.profile)
        rig.local.advance(0); assertTrue(rig.natives.isEmpty()); assertNull(rig.local.audio(microphone))
        assertTrue(rig.local.chooseAudio(rig.lease, MediaLocalConsent(true, true, true, false), 0))
        rig.local.advance(0); rig.local.advance(0); assertEquals(1, rig.natives.size)
        assertEquals(listOf(rig.lease), rig.endpoints.prepared); assertNull(rig.local.audio(speakers))
        rig.admit(); val route = checkNotNull(rig.local.audio(speakers))
        assertEquals(rig.lease, route.lease); assertSame(rig.credential, route.credential)
        assertEquals(23000, route.port); assertEquals(rig.profile, route.profile)
        assertNull(rig.local.audioOffered(speakers)); assertNull(rig.local.audioOffered(microphone))
        assertNull(rig.local.display()); assertNull(rig.local.camera())
        rig.close(); assertNull(rig.local.audio(speakers)); assertNull(rig.local.audioOffered(speakers))
        assertFalse(rig.local.chooseAudio(rig.lease, MediaLocalConsent(true, true, true, false), 2))
    }
    @Test fun t984AllLocalConsentCombinationsRequireExplicitForegroundOrSharedBackgroundChoice() {
        for (background in listOf(false, true)) for (mask in 0..15) {
            val rig = SpeakerRig(background); rig.offer()
            val choice = MediaLocalConsent(mask and 1 != 0, mask and 2 != 0, mask and 4 != 0, mask and 8 != 0)
            assertTrue(rig.local.chooseAudio(rig.lease, choice, 0)); rig.local.advance(0)
            val allowed = choice.enabled && choice.permission && (choice.foreground || (background && choice.backgroundAllowed))
            assertEquals(allowed, rig.natives.isNotEmpty())
            assertEquals(if (choice.enabled) 0 else 1, rig.endpoints.rejected.size)
            rig.local.stop(0); rig.finish(1); assertTrue(rig.local.retired())
        }
    }
    @Test fun t984ActualEndpointAndPublishedConsumerProofJoinInEitherOrder() {
        for (consumerFirst in listOf(false, true)) {
            val rig = SpeakerRig(); rig.start(); rig.admit(); assertNotNull(rig.local.audio(speakers))
            assertFalse(rig.local.audioConsumerRetired(rig.lease, 1)); rig.close(); rig.local.stop(2)
            if (consumerFirst) {
                assertTrue(rig.local.audioConsumerRetired(rig.lease, 2)); assertFalse(rig.local.retired()); rig.finish(3)
            } else {
                rig.finish(3); assertFalse(rig.local.retired()); assertTrue(rig.endpoints.retired.isEmpty())
                assertTrue(rig.local.audioConsumerRetired(rig.lease, 3))
            }
            assertTrue(rig.local.retired()); assertFalse(rig.local.quarantined())
            assertEquals(listOf(rig.lease), rig.endpoints.retired)
            assertFalse(rig.local.audioConsumerRetired(rig.lease, 3))
            rig.endpoints.state(rig.lease, ConnectionResourceState.RETIRED); rig.local.advance(4)
            assertFalse(rig.local.audioConsumerRetired(rig.lease, 4))
        }
    }
    @Test fun t984UnpublishedAndDeniedRoutesRetireWithoutInventingExternalConsumerProof() {
        val denied = SpeakerRig(); denied.offer()
        denied.local.chooseAudio(denied.lease, MediaLocalConsent(true, true, false, false), 0)
        assertTrue(denied.natives.isEmpty()); assertEquals(listOf(denied.lease), denied.endpoints.retired)
        val unpublished = SpeakerRig(); unpublished.start(); unpublished.admit(); unpublished.close(); unpublished.finish(3)
        assertEquals(listOf(unpublished.lease), unpublished.endpoints.retired)
        assertFalse(unpublished.local.audioConsumerRetired(unpublished.lease, 3))
    }
    @Test fun t984StalledOriginalSpeakerDoesNotStopDisplayAndForeignProofCannotFreeItsSlot() {
        val rig = SpeakerRig()
        rig.endpoints.events.add(ConnectionSessionEvent.Offered(rig.display, rig.credential, ConnectionManagement.Display))
        rig.local.advance(0); rig.endpoints.state(rig.display, ConnectionResourceState.ADMITTED); rig.local.advance(0)
        rig.start(); rig.admit(); rig.local.audio(speakers); rig.close(); rig.finish(7)
        assertTrue(rig.local.quarantined()); assertNotNull(rig.local.display()); assertTrue(rig.endpoints.retired.isEmpty())
        val random = java.util.Random(984)
        repeat(128) { assertFalse(rig.local.audioConsumerRetired(rig.lease.copy(owner = random.nextLong() or Long.MIN_VALUE), 7)) }
        assertTrue(rig.local.audioConsumerRetired(rig.lease, 7)); assertFalse(rig.local.quarantined())
        assertEquals(listOf(rig.lease), rig.endpoints.retired); assertNotNull(rig.local.display())
    }
    @Test fun t984OriginalAuthorityExpiryAndRevocationPreventLateEndpointCreation() {
        val rig = SpeakerRig(); rig.offer(); rig.deadline = 1
        assertFalse(rig.local.chooseAudio(rig.lease, MediaLocalConsent(true, true, true, false), 1))
        rig.local.advance(1); assertTrue(rig.natives.isEmpty()); assertNull(rig.local.audioOffered(speakers))
        assertEquals(listOf(rig.lease), rig.endpoints.cancelled); assertEquals(listOf(rig.lease), rig.endpoints.retired)
        val active = SpeakerRig(); active.start(); active.admit(); active.local.audio(speakers)
        active.current = false; active.local.advance(2)
        assertNull(active.local.audio(speakers)); active.finish(3)
        assertTrue(active.endpoints.retired.isEmpty())
        assertTrue(active.local.audioConsumerRetired(active.lease, 3))
    }
    @Test fun t984ForegroundOrPermissionLossStopsOnlyOriginalSpeakerAndBackgroundNeedsBothSides() {
        val foreground = SpeakerRig(); foreground.start(); foreground.admit()
        foreground.local.chooseAudio(foreground.lease, MediaLocalConsent(false, true, true, true), 1)
        assertEquals(listOf(foreground.lease), foreground.endpoints.cancelled)
        val background = SpeakerRig(true); background.start(); background.admit()
        background.local.chooseAudio(background.lease, MediaLocalConsent(false, true, true, true), 1)
        assertNotNull(background.local.audio(speakers))
        background.local.chooseAudio(background.lease, MediaLocalConsent(false, false, true, true), 2)
        assertNull(background.local.audio(speakers)); assertEquals(listOf(background.lease), background.endpoints.cancelled)
        val stopped = SpeakerRig(); stopped.start(); stopped.admit()
        stopped.local.chooseAudio(stopped.lease, MediaLocalConsent(true, true, false, false), 1)
        assertTrue(stopped.endpoints.rejected.isEmpty()); assertEquals(listOf(stopped.lease), stopped.endpoints.cancelled)
    }
    @Test fun t984EndpointFailureAndUncertainRetirementPublicationRetainActualOwnership() {
        val rig = SpeakerRig(); rig.start(); rig.admit(); rig.local.audio(speakers)
        rig.natives.single().failure = java.io.IOException("T984 endpoint failed"); rig.local.advance(2)
        assertNull(rig.local.audio(speakers)); rig.finish(3); assertTrue(rig.endpoints.retired.isEmpty())
        assertTrue(rig.local.audioConsumerRetired(rig.lease, 3)); assertEquals(listOf(rig.lease), rig.endpoints.retired)
        var attempts = 0
        val port = object : ConnectionEndpoints by rig.endpoints {
            override fun nativeRetired(lease: ConnectionResourceLease) { attempts++; error("T984 native proof publication failed") }
        }
        val pending = LocalSpeakerOwner(ConnectionSessionEvent.Offered(rig.lease, rig.credential, rig.profile), port, 4096, ConnectionRunLimits(), rig)
        pending.open(0) { error("No consent") }; pending.stop(0)
        assertTrue(pending.quarantined()); assertFalse(pending.retired()); assertEquals(1, attempts)
        pending.advance(1); pending.stop(1); assertEquals(1, attempts)
    }
    @Test fun t984InvalidProfilesLeasePortsAndMonotonicInputsRejectBeforeNativeWork() {
        val rig = SpeakerRig()
        for (buffer in 0..220) assertEquals(buffer in 20..200 && buffer % 10 == 0,
            runCatching { LocalAudioOffer(rig.lease, rig.profile.copy(bufferMs = buffer)) }.isSuccess)
        for (processing in -1..256) assertEquals(processing in 1..2,
            runCatching { LocalAudioOffer(rig.lease, rig.profile.copy(processing = processing)) }.isSuccess)
        for (port in listOf(Int.MIN_VALUE, -1, 0, 65536, Int.MAX_VALUE))
            assertThrows(IllegalArgumentException::class.java) { LocalAudioRoute(rig.lease, rig.credential, port, rig.profile) }
        assertThrows(IllegalArgumentException::class.java) { LocalAudioOffer(rig.display, rig.profile) }
        assertThrows(IllegalArgumentException::class.java) { LocalAudioOffer(rig.lease.copy(owner = 0), rig.profile) }
        assertThrows(IllegalArgumentException::class.java) { LocalSpeakerOwner(ConnectionSessionEvent.Offered(rig.display, rig.credential, rig.profile), rig.endpoints, 4096, ConnectionRunLimits(), rig) }
        assertThrows(IllegalArgumentException::class.java) { LocalSpeakerOwner(ConnectionSessionEvent.Offered(rig.lease, rig.credential, ConnectionManagement.Display), rig.endpoints, 4096, ConnectionRunLimits(), rig) }
        val owner = LocalSpeakerOwner(ConnectionSessionEvent.Offered(rig.lease, rig.credential, rig.profile), rig.endpoints, 4096, ConnectionRunLimits(), rig)
        for (now in listOf(Long.MIN_VALUE, -1, Long.MAX_VALUE)) assertThrows(IllegalArgumentException::class.java) { owner.open(now) { error("No factory") } }
        owner.open(1) { error("No consent") }
        assertThrows(IllegalStateException::class.java) { owner.open(1) { error("Duplicate") } }
        assertThrows(IllegalArgumentException::class.java) { owner.advance(0) }
        assertThrows(IllegalArgumentException::class.java) { owner.chooseAudio(rig.lease, MediaLocalConsent(true, true, true, false), 0) }
        owner.stop(1)
    }
    @Test fun t984CapabilityCompositionRejectsSpeakerWithoutAdapterAndKeepsMicrophoneUnavailable() {
        for (resource in listOf(speakers, microphone)) {
            val rig = SpeakerRig(); val local = ConnectionLocalResources(rig.endpoints, 4096)
            val lease = rig.lease.copy(key = rig.lease.key.copy(resource = resource))
            rig.endpoints.events.add(ConnectionSessionEvent.Offered(lease, rig.credential, rig.profile)); local.advance(0)
            assertEquals(listOf(lease), rig.endpoints.rejected); assertEquals(listOf(lease), rig.endpoints.retired)
        }
        val invalid = SpeakerRig(); invalid.endpoints.events.add(ConnectionSessionEvent.Offered(invalid.lease, invalid.credential, invalid.profile.copy(bufferMs = 19)))
        invalid.local.advance(0); assertTrue(invalid.natives.isEmpty()); assertEquals(listOf(invalid.lease), invalid.endpoints.rejected)
    }
}
