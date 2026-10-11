package com.blent.connection

import org.junit.Assert.*
import org.junit.Test

private class CameraPresentationRig {
    val model = ConnectionPresentation()
    val token = model.begin()
    val peer = ConnectionPeer.parse("cd".repeat(32))
    fun authenticate() {
        model.event(token, ConnectionWorkerEvent.Peer(cameraLease().connection, peer, false))
        model.event(token, ConnectionWorkerEvent.State(ConnectionWorkerPhase.AUTHENTICATED))
    }
    fun offered(pending: Boolean = false, owner: Long = 1): ConnectionCameraOfferChoice {
        model.event(token, ConnectionWorkerEvent.CameraOffered(LocalCameraOffer(cameraLease(owner), cameraProfile(), pending)))
        return checkNotNull(model.state.value.cameraOffer)
    }
    fun admitted(owner: Long = 1): ConnectionCameraRouteChoice {
        model.event(token, ConnectionWorkerEvent.Camera(cameraRoute(owner)))
        return checkNotNull(model.state.value.camera)
    }
}

class ConnectionCameraPresentationTest {
    @Test fun t941HigherOwnerMayChangeProfileWithoutStaleOfferBlockingIt() {
        val model = ConnectionCameraPresentation(); val ticket = cameraLease().connection
        model.offered(ticket, LocalCameraOffer(cameraLease(), cameraProfile(), true))
        val next = cameraProfile().copy(width = 1280, height = 720)
        model.offered(ticket, LocalCameraOffer(cameraLease(2), next, false))
        assertEquals(cameraLease(2), model.offer?.lease)
        assertEquals(next, model.offer?.offer?.profile)
        model.admitted(ticket, cameraRoute(3).copy(profile = cameraProfile().copy(lens = 0)))
        assertEquals(cameraLease(3), model.route?.lease)
    }
    @Test fun t941PermissionCallbacksNeedExactCurrentPublicationAndOriginalWorker() {
        val rig = CameraPresentationRig(); rig.authenticate(); val initial = rig.offered()
        val context = CameraConsentChoice(true, true)
        assertNull(rig.model.cameraPermission(rig.token, initial, true, context))
        val pending = rig.offered(true)
        assertNotSame(initial, pending); assertNull(rig.model.cameraChoice(rig.token, initial, context))
        assertEquals(ConnectionWorkerCommand.CameraPermission(cameraLease(), true, context),
            rig.model.cameraPermission(rig.token, pending, true, context))
        rig.offered(true); assertSame(pending, rig.model.state.value.cameraOffer)
        rig.offered(false); assertNull(rig.model.cameraPermission(rig.token, pending, true, context))
        val route = rig.admitted()
        assertEquals(ConnectionWorkerCommand.CameraChoice(cameraLease(), context), rig.model.cameraChoice(rig.token, route, context))
        assertNull(rig.model.cameraChoice(rig.token, ConnectionCameraRouteChoice(route.route), context))
        val next = rig.model.begin(); rig.model.event(next, ConnectionWorkerEvent.Peer(cameraLease().connection, rig.peer, true))
        rig.model.event(next, ConnectionWorkerEvent.State(ConnectionWorkerPhase.AUTHENTICATED))
        rig.model.event(next, ConnectionWorkerEvent.Camera(cameraRoute()))
        val replacement = checkNotNull(rig.model.state.value.camera)
        assertNotSame(route, replacement)
        assertNull(rig.model.cameraChoice(next, route, context)); assertNull(rig.model.cameraChoice(rig.token, replacement, context))
        rig.model.event(rig.token, ConnectionWorkerEvent.CameraClosed(cameraLease()))
        assertSame(replacement, rig.model.state.value.camera)
    }
    @Test fun t941CameraClosureIsIndependentAndClosedOwnersCannotReplay() {
        val rig = CameraPresentationRig(); rig.authenticate()
        rig.model.event(rig.token, ConnectionWorkerEvent.Display(displayRoute()))
        val display = rig.model.state.value.display
        val route = rig.admitted(2)
        rig.model.event(rig.token, ConnectionWorkerEvent.CameraClosed(cameraLease()))
        assertSame(route, rig.model.state.value.camera)
        val wrong = cameraLease(2).copy(connection = cameraLease().connection.copy(epoch = 2))
        rig.model.event(rig.token, ConnectionWorkerEvent.CameraClosed(wrong))
        assertSame(route, rig.model.state.value.camera)
        rig.model.event(rig.token, ConnectionWorkerEvent.CameraClosed(cameraLease(2)))
        assertNull(rig.model.state.value.camera); assertSame(display, rig.model.state.value.display)
        rig.model.event(rig.token, ConnectionWorkerEvent.Camera(cameraRoute(2)))
        rig.model.event(rig.token, ConnectionWorkerEvent.CameraOffered(LocalCameraOffer(cameraLease(2), cameraProfile(), true)))
        assertNull(rig.model.state.value.camera); assertNull(rig.model.state.value.cameraOffer)
        val next = rig.offered(true, 3)
        assertEquals(cameraLease(3), next.lease); assertSame(display, rig.model.state.value.display)
    }
    @Test fun t941StopRevokeEndedAndPeerChangesClearCameraImmediately() {
        for (action in 0..4) {
            val rig = CameraPresentationRig(); rig.authenticate(); val route = rig.admitted()
            when (action) {
                0 -> rig.model.stop(rig.token)
                1 -> rig.model.ended(rig.token, null)
                2 -> rig.model.revoke(rig.token, checkNotNull(rig.model.state.value.peer))
                3 -> rig.model.event(rig.token, ConnectionWorkerEvent.State(ConnectionWorkerPhase.CLOSING))
                4 -> rig.model.event(rig.token, ConnectionWorkerEvent.Pairing(cameraLease().connection, rig.peer))
            }
            assertNull(rig.model.state.value.camera); assertNull(rig.model.state.value.cameraOffer)
            assertNull(rig.model.cameraChoice(rig.token, route, CameraConsentChoice(true, true)))
            rig.model.retired(rig.token); assertNull(rig.model.state.value.camera)
        }
    }
    @Test fun t941ForeignAndUnauthenticatedMetadataCannotExposeCamera() {
        val rig = CameraPresentationRig()
        rig.model.event(rig.token, ConnectionWorkerEvent.Camera(cameraRoute()))
        assertNull(rig.model.state.value.camera); rig.authenticate()
        val foreign = cameraLease().copy(connection = cameraLease().connection.copy(epoch = 2))
        rig.model.event(rig.token, ConnectionWorkerEvent.CameraOffered(LocalCameraOffer(foreign, cameraProfile(), true)))
        rig.model.event(rig.token, ConnectionWorkerEvent.Camera(cameraRoute().copy(lease = foreign)))
        assertNull(rig.model.state.value.camera); assertNull(rig.model.state.value.cameraOffer)
        val choice = rig.offered(true)
        val malformed = LocalCameraOffer(cameraLease(), cameraProfile().copy(lens = 0), true)
        rig.model.event(rig.token, ConnectionWorkerEvent.CameraOffered(malformed))
        rig.model.event(rig.token, ConnectionWorkerEvent.Camera(cameraRoute().copy(profile = malformed.profile)))
        assertSame(choice, rig.model.state.value.cameraOffer); assertNull(rig.model.state.value.camera)
        val random = java.util.Random(941)
        repeat(128) {
            val copy = ConnectionCameraOfferChoice(choice.offer)
            assertNull(rig.model.cameraPermission(Any(), choice, random.nextBoolean(), CameraConsentChoice(true, true)))
            assertNull(rig.model.cameraPermission(rig.token, copy, true, CameraConsentChoice(true, true)))
        }
    }
}
