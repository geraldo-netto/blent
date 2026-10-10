package com.blent.connection

import org.junit.Assert.*
import org.junit.Test

private class CameraConsentRig(background: Boolean = false) : CameraConsentAuthority {
    val lease = ConnectionResourceLease(ConnectionTicket(1, ConnectionPeer.parse("a".repeat(64))),
        ConnectionManagement.Key(ConnectionManagement.Resource.CAMERA, 1, 1), 1)
    val offer = ConnectionSessionEvent.Offered(lease, ConnectionManagement.Credential(ByteArray(32) { 42 }),
        ConnectionManagement.Camera(1280, 720, 30, 3000, 1000, 150, 0, background, true))
    var owned = true
    var active = false
    val consent = ConnectionCameraConsent(offer, this)
    override fun owns(lease: ConnectionResourceLease) = owned && lease == this.lease
    override fun admitted(lease: ConnectionResourceLease) = active && owns(lease)
    fun choice(permission: Boolean = false, foreground: Boolean = true, enabled: Boolean = true, backgroundAllowed: Boolean = false) =
        CameraConsentChoice(foreground, permission, enabled, backgroundAllowed)
    fun request(): CameraPermissionRequest = (consent.update(choice()) as CameraConsentAction.Permission).request
}

class ConnectionCameraConsentTest {
    @Test fun t934PendingPermissionNeverPreparesOrAllowsCapture() {
        val rig = CameraConsentRig()
        assertNull(rig.consent.pending()); assertFalse(rig.consent.closing())
        val prompt = rig.request()
        assertSame(prompt, rig.consent.pending())
        assertEquals(rig.lease, prompt.lease)
        repeat(32) { assertNull(rig.consent.update(rig.choice())); assertFalse(rig.consent.captureAllowed(rig.lease, rig.choice(true))) }
        val accepted = rig.consent.permissionResult(prompt, true, rig.choice(true)) as CameraConsentAction.Prepare
        assertSame(rig.offer, accepted.offer); assertNull(rig.consent.pending())
        assertNull(rig.consent.update(rig.choice(true)))
        assertFalse(rig.consent.captureAllowed(rig.lease, rig.choice(true)))
        rig.active = true
        assertTrue(rig.consent.captureAllowed(rig.lease, rig.choice(true)))
        assertFalse(rig.consent.captureAllowed(rig.lease.copy(owner = 2), rig.choice(true)))
        assertFalse(rig.consent.captureAllowed(rig.lease, rig.choice(false)))
    }

    @Test fun t934DeniedOrUnprovenGrantRejectsExactLeaseOnce() {
        for ((granted, actual) in listOf(false to false, false to true, true to false)) {
            val rig = CameraConsentRig(); val prompt = rig.request()
            val denied = rig.consent.permissionResult(prompt, granted, rig.choice(actual)) as CameraConsentAction.Reject
            assertEquals(rig.lease, denied.lease); assertEquals(ConnectionManagement.Outcome.PERMISSION_DENIED, denied.outcome)
            assertTrue(rig.consent.closing()); assertNull(rig.consent.pending())
            assertNull(rig.consent.permissionResult(prompt, true, rig.choice(true)))
            assertNull(rig.consent.update(rig.choice(true))); assertNull(rig.consent.stop())
            assertNull(rig.consent.connectionLost()); assertFalse(rig.consent.captureAllowed(rig.lease, rig.choice(true)))
        }
    }

    @Test fun t934PromptIdentityRejectsForeignReconstructedAndReplacementReplies() {
        val rig = CameraConsentRig(); val prompt = rig.request()
        val replacement = CameraConsentRig(); val fresh = replacement.request()
        for (stale in listOf(prompt, CameraPermissionRequest(replacement.lease), CameraPermissionRequest(replacement.lease.copy(owner = 2)))) {
            assertNull(replacement.consent.permissionResult(stale, true, replacement.choice(true)))
            assertSame(fresh, replacement.consent.pending())
        }
        assertTrue(replacement.consent.permissionResult(fresh, true, replacement.choice(true)) is CameraConsentAction.Prepare)
        assertNull(replacement.consent.permissionResult(fresh, false, replacement.choice(false)))
        assertSame(prompt, rig.consent.pending())
    }

    @Test fun t934AuthorityLossAndLocalStopInvalidatePendingReplies() {
        val lost = CameraConsentRig(); val prompt = lost.request(); lost.owned = false
        val action = lost.consent.permissionResult(prompt, true, lost.choice(true)) as CameraConsentAction.Cancel
        assertEquals(ConnectionManagement.Reason.CONNECTION_LOST, action.reason); assertEquals(lost.lease, action.lease)
        assertNull(lost.consent.pending()); assertFalse(lost.consent.captureAllowed(lost.lease, lost.choice(true)))
        val stopped = CameraConsentRig(); val pending = stopped.request()
        val rejection = stopped.consent.stop() as CameraConsentAction.Reject
        assertEquals(ConnectionManagement.Outcome.UNAVAILABLE, rejection.outcome)
        assertNull(stopped.consent.permissionResult(pending, true, stopped.choice(true)))
        val absent = CameraConsentRig(); absent.owned = false
        assertTrue(absent.consent.update(absent.choice(true)) is CameraConsentAction.Cancel)
        val explicit = CameraConsentRig()
        assertTrue(explicit.consent.connectionLost() is CameraConsentAction.Cancel)
    }

    @Test fun t934ForegroundBackgroundAndLocalIntentAreIndependentPermissionGates() {
        val foreground = CameraConsentRig()
        assertNull(foreground.consent.update(foreground.choice(false, false)))
        assertNull(foreground.consent.update(foreground.choice(true, false)))
        val prompt = foreground.request()
        assertTrue(foreground.consent.permissionResult(prompt, true, foreground.choice(true, false)) is CameraConsentAction.Reject)
        for (host in listOf(false, true)) {
            for (local in listOf(false, true)) {
                val rig = CameraConsentRig(host)
                val choice = rig.choice(true, false, backgroundAllowed = local)
                assertEquals(host && local, rig.consent.update(choice) is CameraConsentAction.Prepare)
            }
        }
        val background = CameraConsentRig(true)
        assertNull(background.consent.update(background.choice(false, false, backgroundAllowed = true)))
        assertTrue(background.consent.update(background.choice(true, false, backgroundAllowed = true)) is CameraConsentAction.Prepare)
        background.active = true
        assertTrue(background.consent.captureAllowed(background.lease, background.choice(true, false, backgroundAllowed = true)))
        assertNull(background.consent.update(background.choice(true, false, backgroundAllowed = true)))
        assertTrue(background.consent.update(background.choice(true, false)) is CameraConsentAction.Cancel)
        val disabled = CameraConsentRig()
        assertTrue(disabled.consent.update(disabled.choice(true, enabled = false)) is CameraConsentAction.Reject)
    }

    @Test fun t934PermissionLossAndStopAfterPreparationRevokeWithoutEarlyRetirement() {
        val rig = CameraConsentRig()
        assertTrue(rig.consent.update(rig.choice(true)) is CameraConsentAction.Prepare)
        rig.active = true
        val revoked = rig.consent.update(rig.choice(false)) as CameraConsentAction.Cancel
        assertEquals(ConnectionManagement.Reason.PERMISSION_REVOKED, revoked.reason)
        assertTrue(rig.owned); assertTrue(rig.active) // Policy cannot manufacture native retirement.
        assertFalse(rig.consent.captureAllowed(rig.lease, rig.choice(true)))
        val stop = CameraConsentRig(); stop.consent.update(stop.choice(true))
        assertEquals(ConnectionManagement.Reason.REQUESTED, (stop.consent.stop() as CameraConsentAction.Cancel).reason)
        val local = CameraConsentRig(); val prompt = local.request()
        assertTrue(local.consent.permissionResult(prompt, true, local.choice(true, enabled = false)) is CameraConsentAction.Reject)
        val inactive = CameraConsentRig(); inactive.consent.update(inactive.choice(true)); inactive.owned = false
        assertFalse(inactive.consent.captureAllowed(inactive.lease, inactive.choice(true)))
    }

    @Test fun t934ConstructedLeaseAndProfileBoundsFailBeforeAnyPreparation() {
        val rig = CameraConsentRig()
        for (value in listOf(Long.MIN_VALUE, -1, 0)) {
            val leases = listOf(rig.lease.copy(owner = value), rig.lease.copy(connection = rig.lease.connection.copy(epoch = value)),
                rig.lease.copy(key = rig.lease.key.copy(request = value)), rig.lease.copy(key = rig.lease.key.copy(generation = value)))
            for (lease in leases) assertThrows(IllegalArgumentException::class.java) { ConnectionCameraConsent(rig.offer.copy(lease = lease), rig) }
        }
        assertThrows(IllegalArgumentException::class.java) { ConnectionCameraConsent(rig.offer.copy(profile = ConnectionManagement.Display), rig) }
        assertThrows(IllegalArgumentException::class.java) { ConnectionCameraConsent(rig.offer.copy(lease = rig.lease.copy(key = rig.lease.key.copy(resource = ConnectionManagement.Resource.DISPLAY))), rig) }
        for (width in listOf(Int.MIN_VALUE, -1, 0, 159, 161, 1921, Int.MAX_VALUE)) {
            val profile = (rig.offer.profile as ConnectionManagement.Camera).copy(width = width)
            assertThrows(IllegalArgumentException::class.java) { ConnectionCameraConsent(rig.offer.copy(profile = profile), rig) }
        }
        val edge = rig.lease.copy(owner = Long.MAX_VALUE, connection = rig.lease.connection.copy(epoch = Long.MAX_VALUE), key = rig.lease.key.copy(request = Long.MAX_VALUE, generation = Long.MAX_VALUE))
        assertNull(ConnectionCameraConsent(rig.offer.copy(lease = edge), object : CameraConsentAuthority {
            override fun owns(lease: ConnectionResourceLease) = true
            override fun admitted(lease: ConnectionResourceLease) = false
        }).update(rig.choice(false, false)))
    }
    private fun rejectedProfile(rig: CameraConsentRig, profile: ConnectionManagement.Camera) {
        assertThrows(IllegalArgumentException::class.java) { ConnectionCameraConsent(rig.offer.copy(profile = profile), rig) }
    }

    @Test fun t934EveryCameraParameterRejectsInvalidAndOutOfBoundValues() {
        val rig = CameraConsentRig(); val profile = rig.offer.profile as ConnectionManagement.Camera
        for (value in listOf(Int.MIN_VALUE, -1, 0, 119, 121, 1081, Int.MAX_VALUE)) rejectedProfile(rig, profile.copy(height = value))
        for (value in listOf(Int.MIN_VALUE, -1, 0, 4, 31, Int.MAX_VALUE)) rejectedProfile(rig, profile.copy(fps = value))
        for (value in listOf(Int.MIN_VALUE, -1, 0, 255, 20001, Int.MAX_VALUE)) {
            rejectedProfile(rig, profile.copy(bitrate = value)); rejectedProfile(rig, profile.copy(minBitrate = value))
        }
        for (value in listOf(Int.MIN_VALUE, -1, 0, 49, 2001, Int.MAX_VALUE)) rejectedProfile(rig, profile.copy(freshnessMs = value))
        for (value in listOf(Int.MIN_VALUE, -1, 2, Int.MAX_VALUE)) rejectedProfile(rig, profile.copy(lens = value))
    }

    @Test fun t934SeededInvalidOffersAndRepeatedStaleRepliesRemainBounded() {
        val random = java.util.Random(934)
        repeat(128) {
            val rig = CameraConsentRig(); val prompt = rig.request()
            val replacement = CameraConsentRig(); val fresh = replacement.request()
            assertNull(replacement.consent.permissionResult(prompt, random.nextBoolean(), replacement.choice(random.nextBoolean())))
            assertSame(fresh, replacement.consent.pending())
            val invalid = (rig.offer.profile as ConnectionManagement.Camera).copy(width = random.nextInt(160))
            rejectedProfile(rig, invalid)
            rig.consent.stop()
            assertNull(rig.consent.permissionResult(prompt, true, rig.choice(true)))
        }
    }

    @Test fun t934NativePreparationRechecksCurrentConsentBeforeOpeningEndpoints() {
        val rig = CameraConsentRig(); rig.request()
        assertFalse(rig.consent.preparationAllowed(rig.lease, rig.choice(true)))
        assertTrue(rig.consent.update(rig.choice(true)) is CameraConsentAction.Prepare)
        assertTrue(rig.consent.preparationAllowed(rig.lease, rig.choice(true)))
        assertFalse(rig.consent.preparationAllowed(rig.lease.copy(owner = 2), rig.choice(true)))
        assertFalse(rig.consent.preparationAllowed(rig.lease, rig.choice(false)))
        assertFalse(rig.consent.preparationAllowed(rig.lease, rig.choice(true, enabled = false)))
        rig.owned = false
        assertFalse(rig.consent.preparationAllowed(rig.lease, rig.choice(true)))
        rig.owned = true; rig.consent.stop()
        assertFalse(rig.consent.preparationAllowed(rig.lease, rig.choice(true)))
    }

}
