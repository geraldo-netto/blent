package com.blent.connection

import org.junit.Assert.*
import org.junit.Test

class LocalCameraResourceTest {
    @Test fun t935CameraGroupCannotOpenWithoutExplicitConsent() {
        val display = ResourceRig()
        val lease = display.lease.copy(key = display.lease.key.copy(resource = ConnectionManagement.Resource.CAMERA))
        val offer = ConnectionSessionEvent.Offered(lease, display.credential,
            ConnectionManagement.Camera(640, 480, 30, 3000, 1000, 150, 0, false, true))
        val group = LocalResourceGroup(offer, display.endpoint, 4096, ConnectionRunLimits())
        var opened = 0
        group.open(0) { opened++; PromptEndpoint(12345) }
        assertEquals("T935 endpoint opened without camera consent", 0, opened)
    }
    @Test fun t935PreparedCameraOwnsOnePortAndPreservesExactProfileCredential() {
        val rig = CameraGroupRig(); rig.open()
        assertEquals(1, rig.native.size); assertEquals(listOf(rig.lease), rig.display.endpoint.prepared)
        assertNull(rig.group.route()); assertNull(rig.group.cameraRoute())
        rig.admit()
        val route = checkNotNull(rig.group.cameraRoute())
        assertEquals(rig.lease, route.lease); assertSame(rig.offer.credential, route.credential)
        assertEquals(rig.offer.profile, route.profile); assertEquals(21000, route.port)
        assertNull(rig.group.route()); assertTrue(rig.native.single().admitted)
        assertTrue(rig.native.single().taken)
        rig.group.stop(2); assertNull(rig.group.cameraRoute()); assertFalse(rig.group.retired())
        rig.native.single().finish(); rig.group.advance(3)
        assertTrue(rig.group.retired()); assertFalse(rig.group.quarantined())
        assertEquals(listOf(rig.lease), rig.display.endpoint.retired)
        rig.group.advance(4); assertEquals(1, rig.display.endpoint.retired.size)
    }

    @Test fun t935PermissionRevocationDuringBindAndAdmissionCannotPublishOrStartPump() {
        val rig = CameraGroupRig(); rig.open(); rig.choice = rig.choice.copy(permission = false)
        rig.group.advance(1)
        assertNull(rig.group.cameraRoute()); assertTrue(rig.native.single().stopped)
        assertFalse(rig.native.single().admitted); assertFalse(rig.native.single().taken)
        assertFalse(rig.group.retired()); rig.native.single().finish(); rig.group.advance(2)
        assertTrue(rig.group.retired())
        val admitted = CameraGroupRig(); admitted.open(); admitted.display.endpoint.active.add(admitted.lease)
        admitted.choice = admitted.choice.copy(permission = false); admitted.group.admit(1)
        assertNull(admitted.group.cameraRoute()); assertFalse(admitted.native.single().admitted)
        assertEquals(listOf(admitted.lease), admitted.display.endpoint.cancelled)
        admitted.native.single().finish(); admitted.group.advance(2)
        assertTrue(admitted.group.retired())
    }

    @Test fun t935StaleAuthorityAndEndpointFailureLeaveDisplayAlive() {
        val rig = CameraGroupRig(); rig.display.offer(); rig.display.admit(); rig.open(); rig.admit()
        rig.owned = false
        assertNull(rig.group.cameraRoute()); rig.group.advance(2)
        assertTrue(rig.native.single().stopped); assertNotNull(rig.display.local.display())
        rig.native.single().finish(); rig.group.advance(3); assertTrue(rig.group.retired())
        val failed = CameraGroupRig(); failed.display.offer(); failed.display.admit(); failed.open(); failed.admit()
        val error = java.io.IOException("camera endpoint failed")
        failed.native.single().failure = error; failed.group.advance(2)
        assertSame(error, failed.group.failure()); assertNull(failed.group.cameraRoute())
        assertNotNull(failed.display.local.display()); assertFalse(failed.group.retired())
        failed.native.single().finish(); failed.group.advance(3); assertTrue(failed.group.retired())
    }

    @Test fun t935PumpFailureKeepsOneChannelNativeProofAndCannotStopDisplay() {
        val rig = CameraGroupRig(); rig.display.offer(); rig.display.admit(); rig.open(); rig.admit()
        val io = rig.native.single().io
        io.complete(io.read(), -1); rig.group.advance(2)
        assertNull(rig.group.cameraRoute()); assertNotNull(rig.display.local.display())
        assertFalse(rig.group.retired()); assertTrue(rig.group.quarantined())
        rig.native.single().finish(); rig.group.advance(3); assertTrue(rig.group.retired())
    }

    @Test fun t935CancelledConsentNeverCallsNativeFactoryOrGrantsReplacement() {
        val rig = CameraGroupRig(); rig.consent.update(rig.choice); rig.consent.stop()
        var opened = 0; rig.group.open(0) { opened++; PromptEndpoint(12345) }; rig.group.advance(0)
        assertEquals(0, opened); assertTrue(rig.group.retired())
        assertEquals(listOf(rig.lease), rig.display.endpoint.retired)
        rig.group.admit(1); assertNull(rig.group.cameraRoute())
        val pending = CameraGroupRig(); pending.choice = pending.choice.copy(permission = false)
        val prompt = (pending.consent.update(pending.choice) as CameraConsentAction.Permission).request
        pending.group.open(0) { opened++; PromptEndpoint(12345) }
        assertEquals(0, opened)
        pending.consent.permissionResult(prompt, false, pending.choice)
        pending.group.advance(1); assertTrue(pending.group.retired())
    }

    @Test fun t935LocalCameraMetadataAndBlockClockBoundariesRejectWithoutOpening() {
        val rig = CameraGroupRig(); val profile = rig.offer.profile as ConnectionManagement.Camera
        for (port in listOf(Int.MIN_VALUE, -1, 0, 65536, Int.MAX_VALUE)) {
            assertThrows(IllegalArgumentException::class.java) { LocalCameraRoute(rig.lease, rig.offer.credential, port, profile) }
        }
        for (block in listOf(Int.MIN_VALUE, -1, 0, ConnectionWire.MAX_PAYLOAD + 1, Int.MAX_VALUE)) {
            assertThrows(IllegalArgumentException::class.java) { LocalResourceGroup(rig.offer, rig.display.endpoint, block, ConnectionRunLimits()) }
        }
        for (now in listOf(Long.MIN_VALUE, -1, Long.MAX_VALUE)) {
            assertThrows(IllegalArgumentException::class.java) { rig.group.open(now) { error("Native factory must not run") } }
        }
        rig.open(); rig.group.advance(2)
        assertThrows(IllegalArgumentException::class.java) { rig.group.advance(1) }
        assertThrows(IllegalArgumentException::class.java) { rig.group.stop(1) }
        assertThrows(IllegalArgumentException::class.java) { LocalCameraRoute(rig.display.lease, rig.offer.credential, 1, profile) }
        assertThrows(IllegalArgumentException::class.java) { LocalCameraRoute(rig.lease, rig.offer.credential, 1, profile.copy(fps = 0)) }
        assertThrows(IllegalArgumentException::class.java) { LocalResourceGroup(rig.offer.copy(profile = ConnectionManagement.Display), rig.display.endpoint, 1, ConnectionRunLimits()) }
        for (port in listOf(1, 65535)) assertEquals(port, LocalCameraRoute(rig.lease, rig.offer.credential, port, profile).port)
    }

    @Test fun t935SeededMalformedLeaseMetadataRemainsBounded() {
        val rig = CameraGroupRig(); val profile = rig.offer.profile as ConnectionManagement.Camera
        val random = java.util.Random(935)
        for (value in listOf(Long.MIN_VALUE, -1, 0) + List(64) { random.nextLong() or Long.MIN_VALUE }) {
            val leases = listOf(rig.lease.copy(owner = value), rig.lease.copy(connection = rig.lease.connection.copy(epoch = value)),
                rig.lease.copy(key = rig.lease.key.copy(request = value)), rig.lease.copy(key = rig.lease.key.copy(generation = value)))
            for (lease in leases) assertThrows(IllegalArgumentException::class.java) { LocalCameraRoute(lease, rig.offer.credential, 12345, profile) }
        }
        LocalCameraRoute(rig.lease.copy(owner = Long.MAX_VALUE), rig.offer.credential, 12345, profile)
    }

    @Test fun t935AdvanceBeforeOpeningCannotPublishPreparedForEmptyChannels() {
        val rig = CameraGroupRig(); rig.consent.update(rig.choice)
        rig.group.advance(0)
        assertTrue("T935 unopened group published Prepared", rig.display.endpoint.prepared.isEmpty())
        assertTrue(rig.native.isEmpty()); assertNull(rig.group.cameraRoute())
        rig.group.stop(1); rig.group.advance(1)
        assertTrue(rig.group.retired())
    }

}

private class CameraGroupRig {
    val display = ResourceRig()
    val lease = display.lease.copy(key = display.lease.key.copy(resource = ConnectionManagement.Resource.CAMERA))
    val offer = ConnectionSessionEvent.Offered(lease, display.credential,
        ConnectionManagement.Camera(640, 480, 30, 3000, 1000, 150, 0, false, true))
    var owned = true
    var choice = CameraConsentChoice(true, true)
    var broken = false
    val authority = object : CameraConsentAuthority {
        override fun owns(lease: ConnectionResourceLease) = owned && lease == this@CameraGroupRig.lease
        override fun admitted(lease: ConnectionResourceLease) = display.endpoint.current(lease)
    }
    val consent = ConnectionCameraConsent(offer, authority)
    val permit = CameraLocalPermit(consent) { if (broken) throw java.io.IOException("permission source failed"); choice }
    val native = ArrayList<PromptEndpoint>()
    val group = LocalResourceGroup(offer, display.endpoint, 4096, ConnectionRunLimits(10, 5), permit)
    fun open() {
        check(consent.update(choice) is CameraConsentAction.Prepare)
        group.open(0) { PromptEndpoint(21000 + native.size).also { native.add(it) } }
        group.advance(0)
    }
    fun admit() { display.endpoint.active.add(lease); group.admit(1); group.advance(1) }
}

class LocalCameraPermitFailureTest {
    @Test fun t935PermitFailureRevokesOnlyCameraAndKeepsNativeCleanup() {
        val rig = CameraGroupRig(); rig.display.offer(); rig.display.admit(); rig.open(); rig.admit()
        rig.broken = true
        assertNull(rig.group.cameraRoute())
        assertTrue(rig.native.single().stopped)
        assertFalse(rig.group.retired()); assertTrue(rig.group.quarantined())
        assertNotNull(rig.display.local.display())
        rig.native.single().finish(); rig.group.advance(2)
        assertTrue(rig.group.retired())
        assertEquals(listOf(rig.lease), rig.display.endpoint.retired)
    }
}
