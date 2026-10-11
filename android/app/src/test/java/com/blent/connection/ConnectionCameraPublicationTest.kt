package com.blent.connection

import org.junit.Assert.*
import org.junit.Test

class ConnectionCameraPublicationTest {
    @Test fun t940PublicationRetainsExactTransitionsAndDoesNotReplayIdleSnapshots() {
        val events = ArrayList<ConnectionWorkerEvent>(); val publication = ConnectionCameraPublication(events::add)
        val offer = LocalCameraOffer(cameraLease(), cameraProfile(), false)
        publication.update(null, null); publication.update(offer, null); publication.update(offer, null)
        val pending = offer.copy(permissionRequired = true); publication.update(pending, null)
        publication.update(null, cameraRoute()); publication.update(null, cameraRoute().copy(credential = cameraRoute().credential))
        // Different credential objects are distinct capabilities, even with identical synthetic bytes.
        assertTrue(events.take(2).all { it is ConnectionWorkerEvent.CameraOffered })
        assertTrue(events.drop(2).all { it is ConnectionWorkerEvent.Camera })
        assertFalse(events.any { it is ConnectionWorkerEvent.CameraClosed })
        publication.update(LocalCameraOffer(cameraLease(2), cameraProfile(), false), null)
        assertEquals(ConnectionWorkerEvent.CameraClosed(cameraLease()), events[events.lastIndex - 1])
        publication.update(null, null); assertEquals(ConnectionWorkerEvent.CameraClosed(cameraLease(2)), events.last())
        val count = events.size; repeat(32) { publication.update(null, null) }; assertEquals(count, events.size)
    }
    @Test fun t940InvalidOrThrowingPublicationCannotReplayOrOverwriteOwnedTransitions() {
        val offer = LocalCameraOffer(cameraLease(), cameraProfile(), false)
        val emitted = ArrayList<ConnectionWorkerEvent>()
        val publication = ConnectionCameraPublication { emitted.add(it); error("Delivery failed") }
        assertThrows(IllegalArgumentException::class.java) { publication.update(offer, cameraRoute()) }
        assertTrue(emitted.isEmpty())
        assertThrows(IllegalStateException::class.java) { publication.update(offer, null) }
        publication.update(offer, null); assertEquals(1, emitted.size)
        assertThrows(IllegalStateException::class.java) { publication.update(null, null) }
        publication.update(null, null); assertEquals(2, emitted.size)
    }
}
