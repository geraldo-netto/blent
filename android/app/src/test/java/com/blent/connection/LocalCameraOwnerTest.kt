package com.blent.connection

import org.junit.Assert.*
import org.junit.Test

class LocalCameraOwnerTest {
    @Test fun t937StoppingPendingCameraCancelsBeforePublishingNativeRetirement() {
        val rig = TabletSessionRig(capabilities = 3)
        var opened = 0
        val local = ConnectionLocalResources(rig.tablet, 4096, factory = { opened++; error("No consent; native work forbidden") }, cameraAuthority = rig.tablet)
        rig.send(rig.invitation(ConnectionManagement.Resource.CAMERA)); local.advance(0)
        assertNotNull(local.offered()); assertEquals(0, opened)
        local.stop(0)
        assertTrue("T937 pending cleanup attempted nativeRetired before protocol cancellation", local.retired())
        assertFalse(local.quarantined()); assertNull(local.offered())
    }
}

private class CameraOwnerRig : ConnectionResourceAuthority {
    val endpoint = ResourceEndpoints()
    val permissions = ArrayList<ConnectionResourceLease>()
    private val port = object : ConnectionEndpoints by endpoint {
        override fun permissionRequired(lease: ConnectionResourceLease, now: Long) { permissions.add(lease) }
    }
    val displayLease = LocalPumpRig().lease
    val lease = displayLease.copy(key = displayLease.key.copy(resource = ConnectionManagement.Resource.CAMERA))
    val credential = ConnectionManagement.Credential(ByteArray(32) { 42 })
    val profile = ConnectionManagement.Camera(640, 480, 30, 3000, 1000, 150, 0, false, true)
    val native = ArrayList<PromptEndpoint>()
    var valid = true
    var deadline = 1000L
    val local = ConnectionLocalResources(port, 4096, ConnectionRunLimits(10, 5), {
        PromptEndpoint(22000 + native.size).also { native.add(it) }
    }, this)
    override fun owns(lease: ConnectionResourceLease, now: Long): Boolean =
        valid && lease == this.lease && (now < deadline || endpoint.current(lease))
    fun offer() {
        endpoint.events.add(ConnectionSessionEvent.Offered(lease, credential, profile)); local.advance(0)
    }
    fun start() {
        offer(); assertTrue(local.choose(lease, CameraConsentChoice(true, true), 0)); local.advance(0)
    }
    fun admit() { endpoint.state(lease, ConnectionResourceState.ADMITTED); local.advance(1) }
    fun close(now: Long = 2) { endpoint.state(lease, ConnectionResourceState.CLOSING); local.advance(now) }
    fun finish(now: Long) { native.forEach { it.finish() }; local.advance(now) }
}

class LocalCameraControlTest {
    @Test fun t937PendingOffersRequireExplicitCurrentPermissionAndPrepareOnce() {
        val rig = CameraOwnerRig()
        assertNull(rig.local.offered()); assertNull(rig.local.pending()); assertNull(rig.local.camera())
        assertFalse(rig.local.choose(rig.lease, CameraConsentChoice(true, true), 0))
        assertFalse(rig.local.consumerRetired(rig.lease, 0))
        rig.offer(); val initial = checkNotNull(rig.local.offered())
        assertEquals(rig.lease, initial.lease); assertEquals(rig.profile, initial.profile); assertFalse(initial.permissionRequired)
        assertSame(initial, rig.local.offered()); assertTrue(rig.native.isEmpty())
        assertTrue(rig.local.choose(rig.lease, CameraConsentChoice(true, false), 0))
        val prompt = checkNotNull(rig.local.pending()); assertTrue(checkNotNull(rig.local.offered()).permissionRequired)
        assertTrue(rig.local.permissionResult(prompt, true, CameraConsentChoice(true, true), 0))
        rig.local.advance(0); assertEquals(1, rig.native.size); assertEquals(listOf(rig.lease), rig.endpoint.prepared)
        assertFalse(rig.local.permissionResult(prompt, false, CameraConsentChoice(true, false), 0))
        assertFalse(rig.local.permissionResult(CameraPermissionRequest(rig.lease), true, CameraConsentChoice(true, true), 0))
        rig.admit(); assertNull(rig.local.offered()); assertNotNull(rig.local.camera()); assertNull(rig.local.display())
        rig.close(); assertNull(rig.local.camera()); assertNull(rig.local.pending())
        assertFalse(rig.local.choose(rig.lease, CameraConsentChoice(true, true), 2))
    }

    @Test fun t937ActualConsumerAndEndpointProofJoinInEitherOrder() {
        for (consumerFirst in listOf(false, true)) {
            val rig = CameraOwnerRig(); rig.start(); rig.admit(); assertNotNull(rig.local.camera())
            assertFalse(rig.local.consumerRetired(rig.lease, 1))
            rig.close(); rig.local.stop(2)
            if (consumerFirst) {
                assertTrue(rig.local.consumerRetired(rig.lease, 2)); assertFalse(rig.local.retired())
                assertTrue(rig.endpoint.retired.isEmpty()); rig.finish(3)
            } else {
                rig.finish(3); assertFalse(rig.local.retired()); assertTrue(rig.endpoint.retired.isEmpty())
                assertTrue(rig.local.consumerRetired(rig.lease, 3))
            }
            assertTrue(rig.local.retired()); assertFalse(rig.local.quarantined())
            assertEquals(listOf(rig.lease), rig.endpoint.retired)
            assertFalse(rig.local.consumerRetired(rig.lease, 3))
            rig.endpoint.state(rig.lease, ConnectionResourceState.RETIRED); rig.local.advance(4)
            assertFalse(rig.local.consumerRetired(rig.lease, 4))
        }
    }

    @Test fun t937PublishedConsumerClaimSurvivesFailedDeliveryAndStuckProofOnlyQuarantinesCamera() {
        val rig = CameraOwnerRig()
        rig.endpoint.events.add(ConnectionSessionEvent.Offered(rig.displayLease, rig.credential, ConnectionManagement.Display))
        rig.local.advance(0); rig.endpoint.state(rig.displayLease, ConnectionResourceState.ADMITTED); rig.local.advance(0)
        rig.start(); rig.admit(); rig.local.camera() // Possible exposure retained even if caller drops metadata.
        rig.close(); rig.finish(7)
        assertTrue(rig.local.quarantined()); assertNotNull(rig.local.display())
        assertTrue(rig.endpoint.retired.isEmpty())
        assertFalse(rig.local.consumerRetired(rig.lease.copy(owner = 2), 7))
        assertTrue(rig.local.consumerRetired(rig.lease, 7))
        assertFalse(rig.local.quarantined()); assertEquals(listOf(rig.lease), rig.endpoint.retired)
        assertNotNull(rig.local.display())
    }

    @Test fun t937DeniedOrUnpublishedCameraHasNoConsumerClaimAndRetiresWithoutOpening() {
        val rig = CameraOwnerRig(); rig.offer()
        rig.local.choose(rig.lease, CameraConsentChoice(true, false), 0)
        val prompt = checkNotNull(rig.local.pending())
        assertTrue(rig.local.permissionResult(prompt, false, CameraConsentChoice(true, false), 0))
        assertTrue(rig.native.isEmpty()); assertNull(rig.local.offered())
        assertEquals(listOf(rig.lease), rig.endpoint.rejected); assertEquals(listOf(rig.lease), rig.endpoint.retired)
        rig.local.stop(0); assertTrue(rig.local.retired())
        val unpublished = CameraOwnerRig(); unpublished.start(); unpublished.admit(); unpublished.close(); unpublished.finish(3)
        assertEquals(listOf(unpublished.lease), unpublished.endpoint.retired)
        assertFalse(unpublished.local.consumerRetired(unpublished.lease, 3))
    }

    @Test fun t937ExpiredPendingGrantAndForeignRepliesCannotOpenNativeEndpoints() {
        val rig = CameraOwnerRig(); rig.offer(); rig.local.choose(rig.lease, CameraConsentChoice(true, false), 0)
        val prompt = checkNotNull(rig.local.pending()); rig.deadline = 1
        assertFalse(rig.local.permissionResult(prompt, true, CameraConsentChoice(true, true), 1))
        assertTrue(rig.native.isEmpty()); assertNull(rig.local.pending()); assertNull(rig.local.offered())
        assertFalse(rig.local.choose(rig.lease.copy(owner = 2), CameraConsentChoice(true, true), 1))
        assertFalse(rig.local.permissionResult(CameraPermissionRequest(rig.lease.copy(owner = 2)), true, CameraConsentChoice(true, true), 1))
        rig.local.advance(1); rig.local.stop(1); assertTrue(rig.local.retired())
    }

    @Test fun t937EndpointFailureClosesCameraAndLateCleanupIsAcceptedAfterWholeStop() {
        val rig = CameraOwnerRig(); rig.start(); rig.admit(); rig.local.camera()
        rig.native.single().failure = java.io.IOException("endpoint failed"); rig.local.advance(2)
        assertNull(rig.local.camera()); assertEquals(listOf(rig.lease), rig.endpoint.cancelled)
        rig.local.stop(2); assertFalse(rig.local.retired())
        rig.finish(3); assertFalse(rig.local.retired())
        assertTrue(rig.local.consumerRetired(rig.lease, 3)); assertTrue(rig.local.retired())
        assertFalse(rig.local.choose(rig.lease, CameraConsentChoice(true, true), 3))
    }

    @Test fun t937NativeProofPublicationFailureQuarantinesWithoutReplayOrFalseRetirement() {
        val rig = CameraOwnerRig(); var attempts = 0
        val endpoint = object : ConnectionEndpoints by rig.endpoint {
            override fun nativeRetired(lease: ConnectionResourceLease) { attempts++; throw java.io.IOException("publication failed") }
        }
        val owner = LocalCameraOwner(ConnectionSessionEvent.Offered(rig.lease, rig.credential, rig.profile), endpoint, 4096, ConnectionRunLimits(), rig)
        owner.open(0) { error("Pending consent must not allocate") }; owner.stop(0)
        assertTrue(owner.quarantined()); assertFalse(owner.retired()); assertEquals(1, attempts)
        owner.advance(1); owner.stop(1); assertEquals(1, attempts)
    }

    @Test fun t937MalformedCameraOffersAndClockInputsRejectWithoutFactoryWork() {
        val rig = CameraOwnerRig()
        rig.endpoint.events.add(ConnectionSessionEvent.Offered(rig.lease, rig.credential, rig.profile.copy(fps = 0)))
        rig.local.advance(0); assertTrue(rig.native.isEmpty()); assertEquals(listOf(rig.lease), rig.endpoint.rejected)
        assertThrows(IllegalArgumentException::class.java) { LocalCameraOffer(rig.displayLease, rig.profile, false) }
        assertThrows(IllegalArgumentException::class.java) { LocalCameraOffer(rig.lease, rig.profile.copy(width = Int.MAX_VALUE), false) }
        val owner = LocalCameraOwner(ConnectionSessionEvent.Offered(rig.lease, rig.credential, rig.profile), rig.endpoint, 4096, ConnectionRunLimits(), rig)
        for (now in listOf(Long.MIN_VALUE, -1, Long.MAX_VALUE)) assertThrows(IllegalArgumentException::class.java) { owner.open(now) { error("No native allocation") } }
        owner.open(1) { error("Pending native factory") }
        assertThrows(IllegalStateException::class.java) { owner.open(1) { error("Duplicate factory") } }
        assertThrows(IllegalArgumentException::class.java) { owner.advance(0) }
        assertThrows(IllegalArgumentException::class.java) { owner.choose(rig.lease, CameraConsentChoice(true, true), 0) }
        owner.stop(1); assertFalse(owner.permissionResult(CameraPermissionRequest(rig.lease), true, CameraConsentChoice(true, true), 1))
    }

    @Test fun t937RealPendingAuthorityExpiresBeforeTimerAndAdmittedResourceRemainsCurrent() {
        val rig = TabletSessionRig(capabilities = 3, timeout = 10)
        val lease = rig.offered(ConnectionManagement.Resource.CAMERA)
        assertTrue(rig.tablet.owns(lease, 9)); assertFalse(rig.tablet.owns(lease, 10))
        assertFalse(rig.tablet.owns(lease.copy(owner = 2), 0))
        assertFalse(rig.tablet.owns(lease.copy(key = lease.key.copy(resource = ConnectionManagement.Resource.DISPLAY)), 0))
        assertThrows(IllegalArgumentException::class.java) { rig.tablet.owns(lease, -1) }
        rig.tablet.prepared(lease, 0); rig.pump(); rig.send(ConnectionManagement.Commit(lease.key))
        rig.host.activate(ConnectionWire.CAMERA, 1); rig.pump()
        assertTrue(rig.tablet.owns(lease, 1000))
        rig.tablet.cancel(lease, ConnectionManagement.Reason.REQUESTED); assertFalse(rig.tablet.owns(lease, 0))
        rig.tablet.disconnect(); assertFalse(rig.tablet.owns(lease, 0))
    }
    @Test fun t937SeededForeignConsumerProofsCannotReleaseOriginalClaim() {
        val rig = CameraOwnerRig(); rig.start(); rig.admit(); rig.local.camera(); rig.close()
        val random = java.util.Random(937)
        repeat(128) {
            val foreign = rig.lease.copy(owner = random.nextLong() or Long.MIN_VALUE)
            assertFalse(rig.local.consumerRetired(foreign, 2)); assertTrue(rig.endpoint.retired.isEmpty())
        }
        rig.finish(3); assertTrue(rig.endpoint.retired.isEmpty())
        assertTrue(rig.local.consumerRetired(rig.lease, 3)); assertEquals(listOf(rig.lease), rig.endpoint.retired)
    }

}
