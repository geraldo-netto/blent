package com.blent.connection

import com.blent.connection.ConnectionManagement.Commit
import com.blent.connection.ConnectionManagement.Outcome
import com.blent.connection.ConnectionManagement.Reason
import com.blent.connection.ConnectionManagement.Resource
import com.blent.connection.ConnectionManagement.ResultMessage
import com.blent.connection.ConnectionManagement.Revoke
import org.junit.Assert.*
import org.junit.Test

class TabletSessionLifecycleTest {
    @Test fun t882CancelledPreparedAcceptsLateCommitAsDrainOnly() {
        val rig = TabletSessionRig()
        val lease = rig.offered(Resource.MICROPHONE)
        rig.tablet.prepared(lease, 0); rig.pump()
        rig.tablet.cancel(lease, Reason.REQUESTED); rig.tablet.nativeRetired(lease)
        rig.send(Commit(lease.key)); rig.host.activate(4, 1)
        assertTrue(rig.host.offer(4, byteArrayOf(99)))
        rig.pump()
        assertFalse(rig.tablet.current(lease))
        assertFalse(rig.messages.contains(ResultMessage(lease.key, Outcome.ADMITTED)))
        rig.peerClose(lease); rig.send(Revoke(lease.key, Reason.REQUESTED)); rig.pump()
        assertTrue(rig.messages.contains(ResultMessage(lease.key, Outcome.REVOKED)))
        assertTrue(rig.events().contains(ConnectionSessionEvent.State(lease, ConnectionResourceState.RETIRED)))
    }
    @Test fun t882UnavailableResourceHasNoNativeInvitationAndCanReuseUninstalledGeneration() {
        val rig = TabletSessionRig(capabilities = 0)
        val invitation = rig.invitation()
        rig.send(invitation); rig.pump()
        val closing = rig.tablet.nextEvent() as ConnectionSessionEvent.State
        assertEquals(ConnectionResourceState.CLOSING, closing.state)
        assertTrue(rig.messages.contains(ResultMessage(invitation.key, Outcome.UNAVAILABLE)))
        rig.send(Revoke(invitation.key, Reason.REQUESTED)); rig.pump()
        assertEquals(ConnectionSessionEvent.State(closing.lease, ConnectionResourceState.RETIRED), rig.tablet.nextEvent())
        rig.send(rig.invitation(request = 2)); rig.pump()
        assertTrue(rig.messages.contains(ResultMessage(invitation.key.copy(request = 2), Outcome.UNAVAILABLE)))
    }
    @Test fun t882InstalledReplacementRejectsStaleReadAndLease() {
        val rig = TabletSessionRig()
        val old = rig.admit(Resource.CAMERA)
        assertTrue(rig.host.offer(3, byteArrayOf(1))); rig.pump()
        val oldRead = checkNotNull(rig.tablet.read(old, 3, 1))
        rig.tablet.cancel(old, Reason.REQUESTED)
        rig.peerClose(old); rig.send(Revoke(old.key, Reason.REQUESTED))
        rig.tablet.nativeRetired(old); rig.pump(); rig.events()
        assertTrue(rig.host.retired(3)); rig.host.replaceChannel(3, 2)
        val next = rig.offered(Resource.CAMERA, 2, 2)
        rig.tablet.prepared(next, 0); rig.pump(); rig.send(Commit(next.key)); rig.host.activate(3, 2); rig.pump()
        assertTrue(rig.host.offer(3, byteArrayOf(2))); rig.pump()
        val newRead = checkNotNull(rig.tablet.read(next, 3, 1))
        assertThrows(IllegalArgumentException::class.java) { rig.tablet.consumed(old, 3, oldRead) }
        assertThrows(IllegalArgumentException::class.java) { rig.tablet.consumed(next, 3, oldRead) }
        assertFalse(rig.tablet.offer(old, 3, byteArrayOf(3)))
        assertEquals(2.toByte(), newRead.bytes[newRead.offset]); rig.tablet.consumed(next, 3, newRead)
    }
    @Test fun t882PeerTerminalClosesBothDisplayChannelsButLeavesAudioLive() {
        val rig = TabletSessionRig()
        val display = rig.admit()
        val audio = rig.admit(Resource.SPEAKERS, 2)
        rig.host.close(1, ConnectionWire.RESET); rig.hostDrain(); rig.pump()
        assertFalse(rig.tablet.current(display)); assertTrue(rig.tablet.current(audio))
        assertTrue(rig.messages.contains(Revoke(display.key, Reason.WORKER_FAILED)))
        assertFalse(rig.host.acceptsData(2, 1))
        assertTrue(rig.tablet.offer(audio, 5, byteArrayOf(1))); rig.pump()
    }
    @Test fun t882TimeoutAndPermissionRejectionRemainPerResource() {
        val rig = TabletSessionRig()
        val live = rig.admit()
        val waiting = rig.offered(Resource.CAMERA, 2)
        rig.tablet.permissionRequired(waiting, 0)
        assertThrows(IllegalStateException::class.java) { rig.tablet.permissionRequired(waiting, 0) }
        rig.now = 100; rig.tablet.tick(100); rig.pump()
        assertTrue(rig.messages.contains(Revoke(waiting.key, Reason.TIMED_OUT)))
        assertTrue(rig.tablet.current(live)); assertFalse(rig.tablet.current(waiting))
        rig.tablet.nativeRetired(waiting); rig.send(Revoke(waiting.key, Reason.TIMED_OUT)); rig.pump()
        assertTrue(rig.messages.contains(ResultMessage(waiting.key, Outcome.REVOKED)))
    }
    @Test fun t882ReadableEventsCoalesceAndReannounceOnlyUnconsumedRemainder() {
        val rig = TabletSessionRig()
        val lease = rig.admit()
        repeat(128) { assertTrue(rig.host.offer(1, byteArrayOf(it.toByte()))); rig.pump() }
        assertEquals(listOf(ConnectionSessionEvent.Readable(lease, 1)), rig.events())
        val first = checkNotNull(rig.tablet.read(lease, 1, 64))
        assertNull(rig.tablet.nextEvent()); rig.tablet.consumed(lease, 1, first)
        assertEquals(ConnectionSessionEvent.Readable(lease, 1), rig.tablet.nextEvent())
        val second = checkNotNull(rig.tablet.read(lease, 1, 64))
        assertEquals(64.toByte(), second.bytes[second.offset]); rig.tablet.consumed(lease, 1, second)
        assertNull(rig.tablet.nextEvent()); assertNull(rig.tablet.read(lease, 1, 1))
    }
}
