package com.blent.connection

import com.blent.connection.ConnectionManagement.Commit
import com.blent.connection.ConnectionManagement.Outcome
import com.blent.connection.ConnectionManagement.Prepared
import com.blent.connection.ConnectionManagement.Reason
import com.blent.connection.ConnectionManagement.Resource
import com.blent.connection.ConnectionManagement.ResultMessage
import com.blent.connection.ConnectionManagement.Revoke
import org.junit.Assert.*
import org.junit.Test

class TabletSessionTest {
    @Test fun t882AdmissionRequiresPreparationAndCommitAcrossPartialRecords() {
        val rig = TabletSessionRig(quantum = 7)
        val lease = rig.offered()
        assertFalse(rig.tablet.current(lease))
        assertFalse(rig.tablet.offer(lease, 1, byteArrayOf(1)))
        rig.tablet.permissionRequired(lease, 0); rig.tablet.prepared(lease, 0); rig.pump()
        assertTrue(rig.messages.contains(ResultMessage(lease.key, Outcome.PERMISSION_REQUIRED)))
        assertTrue(rig.messages.contains(Prepared(lease.key)))
        rig.send(Commit(lease.key)); rig.host.activate(1, 1); rig.host.activate(2, 1); rig.pump()
        assertTrue(rig.tablet.current(lease))
        assertTrue(rig.messages.contains(ResultMessage(lease.key, Outcome.ADMITTED)))
        assertTrue(rig.tablet.offer(lease, 1, byteArrayOf(5, 6)))
        rig.pump(); assertArrayEquals(byteArrayOf(5, 6), rig.payloads.single().second)
        assertFalse(rig.tablet.offer(lease, 3, byteArrayOf(1)))
    }
    @Test fun t882ExpiredCommitDrainsAndNotifiesHostWithoutMediaAuthority() {
        val rig = TabletSessionRig()
        val lease = rig.offered(Resource.CAMERA)
        rig.tablet.prepared(lease, 0); rig.pump()
        rig.now = 100
        rig.send(Commit(lease.key)); rig.host.activate(3, 1); rig.pump()
        assertFalse(rig.tablet.current(lease))
        assertTrue("Expired Commit must notify host terminal coordinator", rig.messages.contains(Revoke(lease.key, Reason.TIMED_OUT)))
        assertFalse(rig.messages.contains(ResultMessage(lease.key, Outcome.ADMITTED)))
    }
    @Test fun t882ReplacementOfferWaitsForActualAckWriteCompletion() {
        val rig = TabletSessionRig()
        val lease = rig.offered()
        rig.tablet.reject(lease, Outcome.PERMISSION_DENIED)
        rig.tablet.nativeRetired(lease)
        rig.send(Revoke(lease.key, Reason.REQUESTED))
        val held = holdAck(rig, lease)
        rig.send(rig.invitation(request = 2))
        assertFalse(rig.events().any { it is ConnectionSessionEvent.Offered })
        assertNull(rig.tablet.nextFrame(0))
        rig.tablet.written(held, 0)
        val events = rig.events()
        assertEquals(ConnectionSessionEvent.State(lease, ConnectionResourceState.RETIRED), events[0])
        val replacement = (events[1] as ConnectionSessionEvent.Offered).lease
        assertEquals(2L, replacement.key.request); assertEquals(1L, replacement.key.generation)
        assertNotEquals(lease.owner, replacement.owner)
    }
    @Test fun t882DuplicateDeferredOfferFailsClosedWithoutPrematureReplacement() {
        val rig = TabletSessionRig()
        val lease = rig.offered()
        rig.tablet.reject(lease, Outcome.BUSY); rig.tablet.nativeRetired(lease)
        rig.send(Revoke(lease.key, Reason.REQUESTED))
        val ack = holdAck(rig, lease)
        rig.send(rig.invitation(request = 2))
        assertThrows(IllegalStateException::class.java) { rig.send(rig.invitation(request = 3)) }
        assertFalse(rig.events().any { it is ConnectionSessionEvent.Offered })
        rig.tablet.written(ack, 0); rig.tablet.transportRetired()
        assertFalse(rig.events().any { it is ConnectionSessionEvent.Offered })
    }
    @Test fun t882InvalidDeferredGenerationFailsClosed() {
        val rig = TabletSessionRig()
        val lease = rig.offered()
        rig.tablet.reject(lease, Outcome.FAILED); rig.tablet.nativeRetired(lease)
        rig.send(Revoke(lease.key, Reason.REQUESTED))
        holdAck(rig, lease)
        assertThrows(IllegalArgumentException::class.java) { rig.send(rig.invitation(request = 2, generation = 2)) }
        rig.tablet.transportRetired()
        assertFalse(rig.events().any { it is ConnectionSessionEvent.Offered })
    }
    private fun holdAck(rig: TabletSessionRig, lease: ConnectionResourceLease): ConnectionFrame {
        repeat(128) {
            val frame = checkNotNull(rig.tablet.nextFrame(rig.now))
            rig.receive(frame)
            if (rig.messages.lastOrNull() == ResultMessage(lease.key, Outcome.REVOKED)) return frame
            rig.tablet.written(frame, rig.now)
        }
        error("T882 ACK not published")
    }
    @Test fun t882BorrowStallAndCameraCancellationLeaveDisplayLive() {
        val rig = TabletSessionRig()
        val display = rig.admit()
        val camera = rig.admit(Resource.CAMERA, 2)
        assertTrue(rig.host.offer(3, ByteArray(4096) { 3 })); rig.pump()
        val borrowed = checkNotNull(rig.tablet.read(camera, 3, 64))
        assertEquals(64, borrowed.length); assertNull(rig.tablet.read(camera, 3, 64))
        assertTrue(rig.host.offer(1, byteArrayOf(9))); rig.pump()
        val control = checkNotNull(rig.tablet.read(display, 1, 1))
        assertEquals(9.toByte(), control.bytes[control.offset]); rig.tablet.consumed(display, 1, control)
        rig.tablet.cancel(camera, Reason.REQUESTED); rig.tablet.cancel(camera, Reason.REQUESTED)
        rig.peerClose(camera); rig.send(Revoke(camera.key, Reason.REQUESTED)); rig.pump()
        assertFalse(rig.messages.contains(ResultMessage(camera.key, Outcome.REVOKED)))
        rig.tablet.consumed(camera, 3, borrowed)
        rig.tablet.nativeRetired(camera); rig.pump()
        assertTrue(rig.messages.contains(ResultMessage(camera.key, Outcome.REVOKED)))
        assertTrue(rig.tablet.current(display)); assertFalse(rig.tablet.current(camera))
        assertTrue(rig.tablet.offer(display, 1, byteArrayOf(8))); rig.pump()
    }
    @Test fun t882DisconnectWaitsForBothActualRetirementsAndRetainsBorrowMemory() {
        val rig = TabletSessionRig()
        val lease = rig.admit(Resource.SPEAKERS)
        assertTrue(rig.host.offer(5, byteArrayOf(7, 8))); rig.pump()
        val read = checkNotNull(rig.tablet.read(lease, 5, 1))
        assertTrue(rig.tablet.offer(lease, 5, byteArrayOf(6)))
        val write = checkNotNull(rig.tablet.nextFrame(0))
        rig.tablet.disconnect(); rig.tablet.disconnect(); rig.tablet.nativeRetired(lease)
        assertFalse(rig.events().any { it == ConnectionSessionEvent.State(lease, ConnectionResourceState.RETIRED) })
        assertEquals(7.toByte(), read.bytes[read.offset])
        rig.tablet.transportRetired()
        assertEquals(listOf(ConnectionSessionEvent.State(lease, ConnectionResourceState.RETIRED)), rig.events())
        assertThrows(IllegalArgumentException::class.java) { rig.tablet.written(write, 0) }
        assertThrows(IllegalStateException::class.java) { rig.tablet.transportRetired() }
        assertFalse(rig.tablet.current(lease))
    }
}
