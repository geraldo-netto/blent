package com.blent.connection

import com.blent.MediaLocalConsent
import org.junit.Assert.*
import org.junit.Test

private class MediaPrompt(val lease: ConnectionResourceLease)

private class MediaConsentRig(
    resource: ConnectionManagement.Resource = ConnectionManagement.Resource.MICROPHONE,
    background: Boolean = false,
    prompt: Boolean = true,
    original: ConnectionResourceLease? = null,
) : ConnectionMediaConsentAuthority {
    val lease = original ?: ConnectionResourceLease(ConnectionTicket(1, ConnectionPeer.parse("b".repeat(64))),
        ConnectionManagement.Key(resource, 1, 1), 1)
    var owned = true
    var active = false
    var prompts = 0
    val consent = ConnectionMediaConsent(lease, background, prompt, this) { prompts++; MediaPrompt(it) }
    override fun owns(lease: ConnectionResourceLease) = owned && lease == this.lease
    override fun admitted(lease: ConnectionResourceLease) = active && owns(lease)
    fun choice(permission: Boolean = false, foreground: Boolean = true, enabled: Boolean = true,
               background: Boolean = false) = MediaLocalConsent(foreground, permission, enabled, background)
    fun request(): MediaPrompt = (consent.update(choice()) as ConnectionMediaConsentAction.Permission).request
}

class ConnectionMediaConsentTest {
    @Test fun t996OriginalOpaquePromptNeedsFreshPermissionAndAdmission() {
        val rig = MediaConsentRig()
        assertNull(rig.consent.pending()); assertFalse(rig.consent.closing())
        val prompt = rig.request()
        assertSame(prompt, rig.consent.pending()); assertEquals(rig.lease, prompt.lease)
        repeat(32) { assertNull(rig.consent.update(rig.choice())); assertEquals(1, rig.prompts) }
        assertNull(rig.consent.permissionResult(MediaPrompt(rig.lease), true, rig.choice(true)))
        assertSame(prompt, rig.consent.pending())
        assertFalse(rig.consent.preparationAllowed(rig.lease, rig.choice(true)))
        assertEquals(ConnectionMediaConsentAction.Prepare, rig.consent.permissionResult(prompt, true, rig.choice(true)))
        assertNull(rig.consent.pending()); assertNull(rig.consent.update(rig.choice(true)))
        assertTrue(rig.consent.preparationAllowed(rig.lease, rig.choice(true)))
        assertFalse(rig.consent.captureAllowed(rig.lease, rig.choice(true)))
        rig.active = true
        assertTrue(rig.consent.captureAllowed(rig.lease, rig.choice(true)))
        assertFalse(rig.consent.captureAllowed(rig.lease.copy(owner = 2), rig.choice(true)))
        assertFalse(rig.consent.preparationAllowed(rig.lease, rig.choice(false)))
    }

    @Test fun t996DeniedUnprovenAndWithdrawnPermissionRepliesCloseOriginalOnce() {
        for ((granted, actual) in listOf(false to false, false to true, true to false)) {
            val rig = MediaConsentRig(); val prompt = rig.request()
            assertEquals(ConnectionMediaConsentAction.Reject(ConnectionManagement.Outcome.PERMISSION_DENIED),
                rig.consent.permissionResult(prompt, granted, rig.choice(actual)))
            closed(rig, prompt)
        }
        for (choice in listOf(MediaLocalConsent(false, true, true, false), MediaLocalConsent(true, true, false, false))) {
            val rig = MediaConsentRig(); val prompt = rig.request()
            assertEquals(ConnectionMediaConsentAction.Reject(ConnectionManagement.Outcome.UNAVAILABLE),
                rig.consent.permissionResult(prompt, true, choice))
            closed(rig, prompt)
        }
    }

    private fun closed(rig: MediaConsentRig, prompt: MediaPrompt) {
        assertTrue(rig.consent.closing()); assertNull(rig.consent.pending())
        assertNull(rig.consent.permissionResult(prompt, true, rig.choice(true)))
        assertNull(rig.consent.update(rig.choice(true))); assertNull(rig.consent.stop())
        assertNull(rig.consent.connectionLost()); assertFalse(rig.consent.captureAllowed(rig.lease, rig.choice(true)))
    }

    @Test fun t996ReconstructedAndEqualCounterReplacementCannotCompleteOriginalPrompt() {
        val random = java.util.Random(996)
        repeat(128) {
            val old = MediaConsentRig(); val oldPrompt = old.request()
            val replacement = MediaConsentRig(); val fresh = replacement.request()
            assertEquals(old.lease, replacement.lease)
            for (stale in listOf(oldPrompt, MediaPrompt(replacement.lease), MediaPrompt(replacement.lease.copy(owner = 2)))) {
                assertNull(replacement.consent.permissionResult(stale, random.nextBoolean(), replacement.choice(random.nextBoolean())))
                assertSame(fresh, replacement.consent.pending())
            }
            assertEquals(ConnectionMediaConsentAction.Prepare, replacement.consent.permissionResult(fresh, true, replacement.choice(true)))
            old.consent.stop(); closed(old, oldPrompt)
        }
    }

    @Test fun t996SpeakerNeverRequestsSdkPermission() {
        val rig = MediaConsentRig(ConnectionManagement.Resource.SPEAKERS, prompt = false)
        assertNull(rig.consent.update(rig.choice(false)))
        assertNull(rig.consent.update(rig.choice(true, foreground = false)))
        assertEquals(0, rig.prompts); assertNull(rig.consent.pending())
        assertEquals(ConnectionMediaConsentAction.Prepare, rig.consent.update(rig.choice(true)))
        rig.active = true
        assertTrue(rig.consent.captureAllowed(rig.lease, rig.choice(true)))
        assertEquals(ConnectionMediaConsentAction.Cancel(ConnectionManagement.Reason.REQUESTED), rig.consent.stop())
    }

    @Test fun t996ForegroundBackgroundAndLocalIntentRemainIndependent() {
        for (hostBackground in listOf(false, true)) {
            for (localBackground in listOf(false, true)) {
                val rig = MediaConsentRig(background = hostBackground)
                val choice = rig.choice(true, foreground = false, background = localBackground)
                assertEquals(hostBackground && localBackground, rig.consent.update(choice) == ConnectionMediaConsentAction.Prepare)
                assertNull(rig.consent.pending())
            }
        }
        val rig = MediaConsentRig(background = true)
        assertNull(rig.consent.update(rig.choice(false, false, background = true)))
        assertEquals(ConnectionMediaConsentAction.Prepare, rig.consent.update(rig.choice(true, false, background = true)))
        assertNull(rig.consent.update(rig.choice(true, false, background = true)))
        assertEquals(ConnectionMediaConsentAction.Cancel(ConnectionManagement.Reason.REQUESTED), rig.consent.update(rig.choice(true, false)))
        val disabled = MediaConsentRig()
        assertEquals(ConnectionMediaConsentAction.Reject(ConnectionManagement.Outcome.UNAVAILABLE), disabled.consent.update(disabled.choice(true, enabled = false)))
    }

    @Test fun t996SourceLossAndPermissionRevocationNeverProveNativeRetirement() {
        val pending = MediaConsentRig(); val prompt = pending.request(); pending.owned = false
        assertEquals(ConnectionMediaConsentAction.Cancel(ConnectionManagement.Reason.CONNECTION_LOST),
            pending.consent.permissionResult(prompt, true, pending.choice(true)))
        closed(pending, prompt)
        val running = MediaConsentRig(); running.consent.update(running.choice(true)); running.active = true
        assertEquals(ConnectionMediaConsentAction.Cancel(ConnectionManagement.Reason.PERMISSION_REVOKED), running.consent.update(running.choice(false)))
        assertTrue(running.owned); assertTrue(running.active)
        assertFalse(running.consent.captureAllowed(running.lease, running.choice(true)))
        val lost = MediaConsentRig(); lost.owned = false
        assertEquals(ConnectionMediaConsentAction.Cancel(ConnectionManagement.Reason.CONNECTION_LOST), lost.consent.update(lost.choice(true)))
        val opened = MediaConsentRig(); opened.consent.update(opened.choice(true)); opened.owned = false
        assertFalse(opened.consent.preparationAllowed(opened.lease, opened.choice(true)))
        val explicit = MediaConsentRig()
        assertEquals(ConnectionMediaConsentAction.Cancel(ConnectionManagement.Reason.CONNECTION_LOST), explicit.consent.connectionLost())
    }

    @Test fun t996EveryLeaseCounterRejectsInvalidBoundsBeforePromptCreation() {
        val lease = MediaConsentRig().lease
        for (value in listOf(Long.MIN_VALUE, -1L, 0L)) {
            val invalid = listOf(lease.copy(owner = value), lease.copy(connection = lease.connection.copy(epoch = value)),
                lease.copy(key = lease.key.copy(request = value)), lease.copy(key = lease.key.copy(generation = value)))
            for (bad in invalid) assertThrows(IllegalArgumentException::class.java) { MediaConsentRig(original = bad) }
        }
        assertThrows(IllegalArgumentException::class.java) { MediaConsentRig(ConnectionManagement.Resource.DISPLAY) }
        for (resource in listOf(ConnectionManagement.Resource.CAMERA, ConnectionManagement.Resource.MICROPHONE, ConnectionManagement.Resource.SPEAKERS)) {
            val edge = lease.copy(owner = Long.MAX_VALUE, connection = lease.connection.copy(epoch = Long.MAX_VALUE),
                key = lease.key.copy(resource = resource, request = Long.MAX_VALUE, generation = Long.MAX_VALUE))
            val rig = MediaConsentRig(original = edge)
            assertEquals(ConnectionMediaConsentAction.Prepare, rig.consent.update(rig.choice(true)))
        }
    }
}
