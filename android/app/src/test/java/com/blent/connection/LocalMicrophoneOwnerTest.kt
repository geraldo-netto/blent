package com.blent.connection

import com.blent.MediaLocalConsent
import org.junit.Assert.*
import org.junit.Test

internal class MicrophoneOwnerRig(val background: Boolean = false) : ConnectionResourceAuthority {
    val endpoints = ResourceEndpoints()
    val display = LocalPumpRig().lease
    val lease = display.copy(key = display.key.copy(resource = ConnectionManagement.Resource.MICROPHONE))
    val credential = ConnectionManagement.Credential(ByteArray(32) { 41 })
    val profile = ConnectionManagement.Audio(1, 40, background)
    val permissions = mutableListOf<ConnectionResourceLease>()
    val port = object : ConnectionEndpoints by endpoints {
        override fun permissionRequired(lease: ConnectionResourceLease, now: Long) { permissions.add(lease) }
    }
    val natives = mutableListOf<PromptEndpoint>()
    var current = true
    var deadline = 1000L
    val local = ConnectionLocalResources(port, 4096, ConnectionRunLimits(10, 5), {
        PromptEndpoint(25000 + natives.size).also { natives.add(it) }
    }, microphoneAuthority = this)
    override fun owns(lease: ConnectionResourceLease, now: Long): Boolean =
        current && lease == this.lease && (now < deadline || endpoints.current(lease))
    fun offer() { endpoints.events.add(ConnectionSessionEvent.Offered(lease, credential, profile)); local.advance(0) }
    fun choose(permission: Boolean = false, foreground: Boolean = true, enabled: Boolean = true, background: Boolean = false) =
        MediaLocalConsent(foreground, permission, enabled, background)
    fun request(): AudioPermissionRequest {
        offer(); assertTrue(local.chooseAudio(lease, choose(), 0)); return checkNotNull(local.pendingAudio())
    }
    fun start() { offer(); local.chooseAudio(lease, choose(true), 0); local.advance(0) }
    fun admit() { endpoints.state(lease, ConnectionResourceState.ADMITTED); local.advance(1) }
    fun finish(now: Long) { natives.forEach { it.finish() }; local.advance(now) }
}

class LocalMicrophoneOwnerTest {
    private val microphone = ConnectionManagement.Resource.MICROPHONE
    @Test fun t997PendingOriginalPermissionOpensNoEndpointAndGrantRechecksFreshContext() {
        val rig = MicrophoneOwnerRig()
        assertNull(rig.local.pendingAudio()); assertNull(rig.local.audioOffered(microphone))
        assertFalse(rig.local.audioPermissionResult(AudioPermissionRequest(rig.lease), true, rig.choose(true), 0))
        val prompt = rig.request()
        assertEquals(rig.lease, prompt.lease); assertEquals(listOf(rig.lease), rig.permissions)
        val offer = checkNotNull(rig.local.audioOffered(microphone))
        assertTrue(offer.permissionRequired); assertSame(offer, rig.local.audioOffered(microphone))
        repeat(32) { rig.local.advance(0); assertTrue(rig.natives.isEmpty()); assertSame(prompt, rig.local.pendingAudio()) }
        assertFalse(rig.local.audioPermissionResult(AudioPermissionRequest(rig.lease), true, rig.choose(true), 0))
        assertTrue(rig.local.audioPermissionResult(prompt, true, rig.choose(true), 0))
        assertNull(rig.local.pendingAudio()); assertFalse(checkNotNull(rig.local.audioOffered(microphone)).permissionRequired)
        assertFalse(rig.local.audioPermissionResult(prompt, true, rig.choose(true), 0))
        rig.local.advance(0); assertEquals(1, rig.natives.size); assertEquals(listOf(rig.lease), rig.endpoints.prepared)
        assertNull(rig.local.audio(microphone)); rig.admit()
        val route = checkNotNull(rig.local.audio(microphone))
        assertEquals(rig.lease, route.lease); assertSame(rig.credential, route.credential); assertEquals(rig.profile, route.profile)
        assertNull(rig.local.audioOffered(microphone)); assertNull(rig.local.audio(ConnectionManagement.Resource.SPEAKERS))
    }

    @Test fun t997DenialUnprovenGrantAndForegroundWithdrawalRetireWithoutCapture() {
        for ((granted, actual) in listOf(false to false, false to true, true to false)) {
            val rig = MicrophoneOwnerRig(); val prompt = rig.request()
            assertTrue(rig.local.audioPermissionResult(prompt, granted, rig.choose(actual), 0))
            assertTrue(rig.natives.isEmpty()); assertEquals(listOf(rig.lease), rig.endpoints.rejected)
            assertEquals(listOf(rig.lease), rig.endpoints.retired); assertNull(rig.local.pendingAudio())
            rig.local.stop(0); assertTrue(rig.local.retired())
        }
        for (choice in listOf(MediaLocalConsent(false, true, true, false), MediaLocalConsent(true, true, false, false))) {
            val rig = MicrophoneOwnerRig(); val prompt = rig.request()
            assertTrue(rig.local.audioPermissionResult(prompt, true, choice, 0)); assertTrue(rig.natives.isEmpty())
            assertEquals(listOf(rig.lease), rig.endpoints.rejected)
        }
    }

    @Test fun t997OriginalExpiryForeignAndReplacementPromptsCannotOpenEndpoint() {
        val rig = MicrophoneOwnerRig(); val prompt = rig.request(); rig.deadline = 1
        assertFalse(rig.local.audioPermissionResult(prompt, true, rig.choose(true), 1))
        rig.local.advance(1); assertNull(rig.local.pendingAudio()); assertTrue(rig.natives.isEmpty())
        assertEquals(listOf(rig.lease), rig.endpoints.cancelled)
        val replacement = MicrophoneOwnerRig(); val fresh = replacement.request()
        val random = java.util.Random(997)
        repeat(128) {
            val foreign = AudioPermissionRequest(replacement.lease.copy(owner = (random.nextLong() and Long.MAX_VALUE).coerceAtLeast(2)))
            assertFalse(replacement.local.audioPermissionResult(foreign, true, replacement.choose(true), 0))
        }
        assertFalse(replacement.local.audioPermissionResult(prompt, true, replacement.choose(true), 0))
        assertSame(fresh, replacement.local.pendingAudio())
        assertTrue(replacement.local.audioPermissionResult(fresh, true, replacement.choose(true), 0))
    }

    @Test fun t997ActualEndpointAndConsumerRetirementRemainIndependentInEitherOrder() {
        for (consumerFirst in listOf(false, true)) {
            val rig = MicrophoneOwnerRig(); rig.start(); rig.admit(); assertNotNull(rig.local.audio(microphone))
            assertFalse(rig.local.audioConsumerRetired(rig.lease, 1))
            rig.endpoints.state(rig.lease, ConnectionResourceState.CLOSING); rig.local.advance(2); rig.local.stop(2)
            if (consumerFirst) {
                assertTrue(rig.local.audioConsumerRetired(rig.lease, 2)); assertFalse(rig.local.retired()); rig.finish(3)
            } else {
                rig.finish(3); assertFalse(rig.local.retired()); assertTrue(rig.endpoints.retired.isEmpty())
                assertTrue(rig.local.audioConsumerRetired(rig.lease, 3))
            }
            assertTrue(rig.local.retired()); assertEquals(listOf(rig.lease), rig.endpoints.retired)
        }
    }

    @Test fun t997HeldMicrophoneCleanupAndPermissionLossLeaveDisplayCurrent() {
        val rig = MicrophoneOwnerRig()
        rig.endpoints.events.add(ConnectionSessionEvent.Offered(rig.display, rig.credential, ConnectionManagement.Display))
        rig.local.advance(0); rig.endpoints.state(rig.display, ConnectionResourceState.ADMITTED); rig.local.advance(0)
        rig.start(); rig.admit(); rig.local.audio(microphone)
        rig.local.chooseAudio(rig.lease, rig.choose(false), 1); rig.finish(7)
        assertNull(rig.local.audio(microphone)); assertTrue(rig.local.quarantined()); assertNotNull(rig.local.display())
        assertFalse(rig.local.audioConsumerRetired(rig.lease.copy(owner = 2), 7))
        assertTrue(rig.local.audioConsumerRetired(rig.lease, 7)); assertFalse(rig.local.quarantined())
        assertNotNull(rig.local.display()); assertEquals(listOf(rig.lease), rig.endpoints.cancelled)
    }

    @Test fun t997FullConsentMatrixUsesIndependentIntentAndBothBackgroundChoices() {
        for (background in listOf(false, true)) for (bits in 0..15) {
            val rig = MicrophoneOwnerRig(background); rig.offer()
            val context = MediaLocalConsent(bits and 1 != 0, bits and 2 != 0, bits and 4 != 0, bits and 8 != 0)
            assertTrue(rig.local.chooseAudio(rig.lease, context, 0)); rig.local.advance(0)
            assertEquals(context.enabled && context.permission && (context.foreground || (background && context.backgroundAllowed)), rig.natives.isNotEmpty())
            rig.local.stop(0); rig.finish(1); assertTrue(rig.local.retired())
        }
    }

    @Test fun t997InvalidAudioProfileAndClockRejectBeforeNativeWork() {
        val rig = MicrophoneOwnerRig()
        val owner = LocalAudioOwner(ConnectionSessionEvent.Offered(rig.lease, rig.credential, rig.profile), rig.port, 4096, ConnectionRunLimits(), rig)
        for (now in listOf(Long.MIN_VALUE, -1, Long.MAX_VALUE))
            assertThrows(IllegalArgumentException::class.java) { owner.open(now) { error("No native work") } }
        owner.open(1) { error("No local intent") }
        assertThrows(IllegalArgumentException::class.java) { owner.audioPermissionResult(AudioPermissionRequest(rig.lease), true, rig.choose(true), 0) }
        assertFalse(owner.chooseAudio(rig.lease.copy(owner = 2), rig.choose(true), 1))
        owner.stop(1); assertTrue(owner.retired())
        assertThrows(IllegalArgumentException::class.java) { LocalAudioOwner(ConnectionSessionEvent.Offered(rig.lease, rig.credential, ConnectionManagement.Display), rig.port, 4096, ConnectionRunLimits(), rig) }
        assertThrows(IllegalArgumentException::class.java) { LocalAudioOffer(rig.lease.copy(key = rig.lease.key.copy(resource = ConnectionManagement.Resource.SPEAKERS)), rig.profile, true) }
        assertThrows(IllegalArgumentException::class.java) { AudioPermissionRequest(rig.display) }
    }
    @Test fun t997LocalRunPreservesFocusedPromptIdentityAndRejectsNewRepliesAfterStop() {
        val rig = MicrophoneOwnerRig()
        val run = ConnectionLocalRun(RuntimeRun(), rig.local)
        assertNull(run.pendingAudio())
        val prompt = rig.request(); assertSame(prompt, run.pendingAudio())
        assertTrue(run.audioPermissionResult(prompt, true, rig.choose(true), 0))
        assertNull(run.pendingAudio()); run.stop(0)
        assertNull(run.pendingAudio()); assertFalse(run.audioPermissionResult(prompt, true, rig.choose(true), 0))
        val unsupported = ConnectionLocalRun(RuntimeRun(), object : LocalConnectionResources {
            override fun advance(now: Long) {}
            override fun display(): LocalDisplayRoute? = null
            override fun stop(now: Long) {}
            override fun retired() = true
            override fun quarantined() = false
        })
        assertNull(unsupported.pendingAudio()); assertFalse(unsupported.audioPermissionResult(prompt, true, rig.choose(true), 0))
    }
}
