package com.blent.connection

import org.junit.Assert.*
import org.junit.Test

class TabletSessionBoundaryTest {
    private fun ticket(epoch: Long = 1) = ConnectionTicket(epoch, ConnectionPeer.parse("cd".repeat(32)))
    private fun session() = TabletSession(ticket(), ConnectionSessionLimits(capacity = 4096), 15)
    private fun rejected(work: () -> Unit) { assertThrows(Exception::class.java) { work() } }
    @Test fun t882InitialOwnerRejectsInvalidIdentityAndCapabilities() {
        for (epoch in listOf(Long.MIN_VALUE, -1, 0)) {
            rejected { TabletSession(ticket(epoch), ConnectionSessionLimits(), 0) }
        }
        for (mask in listOf(Int.MIN_VALUE, -1, 16, Int.MAX_VALUE)) {
            rejected { TabletSession(ticket(), ConnectionSessionLimits(), mask) }
        }
        val owner = session(); assertNull(owner.nextEvent()); rejected { owner.nextFrame(-1) }
        val frame = checkNotNull(owner.nextFrame(0)); assertNull(owner.nextFrame(0))
        assertEquals(ConnectionWire.MANAGEMENT, frame.header.channel)
        assertEquals(ConnectionWire.CREDIT, frame.header.kind)
        rejected { owner.written(frame.copy(), 0) }; owner.written(frame, 0)
        rejected { owner.written(frame, 0) }; assertNull(owner.nextFrame(0))
    }
    private fun malformed(frame: ConnectionFrame) {
        val owner = session()
        rejected { owner.receive(frame, 0) }
        rejected { owner.nextFrame(0) }
        owner.disconnect(); owner.transportRetired(); rejected { owner.transportRetired() }
    }
    @Test fun t882MalformedEnvelopesFailConnectionWithoutLeakingOwnership() {
        val bytes = ByteArray(4)
        val header = ConnectionWire.Header(ConnectionWire.DATA, 6, 1, 1, 0, 1, 0)
        for (offset in listOf(Int.MIN_VALUE, -1, 5, Int.MAX_VALUE)) {
            malformed(ConnectionFrame(header, bytes, offset, 1))
        }
        for (length in listOf(Int.MIN_VALUE, -1, 0, 5, Int.MAX_VALUE)) {
            malformed(ConnectionFrame(header.copy(length = length), bytes, 0, length))
        }
        malformed(ConnectionFrame(header, bytes, 0, 2))
        malformed(ConnectionFrame(header.copy(channel = 0), bytes, 0, 1))
        malformed(ConnectionFrame(header.copy(kind = ConnectionWire.CLOSE, length = 0), bytes, 0, 0))
    }
    @Test fun t882DisconnectRetainsWriterUntilCompletionOrTransportExit() {
        val owner = session(); val frame = checkNotNull(owner.nextFrame(0))
        rejected { owner.transportRetired() }
        owner.disconnect(); owner.disconnect(); owner.written(frame, 0)
        rejected { owner.written(frame, 0) }; owner.transportRetired()
        val abandoned = session(); val loan = checkNotNull(abandoned.nextFrame(0))
        abandoned.disconnect(); abandoned.transportRetired()
        rejected { abandoned.written(loan, 0) }; rejected { abandoned.receive(loan, 0) }
    }
    @Test fun t882UnownedEndpointsNeverGrantDataAuthority() {
        val owner = session()
        val key = ConnectionManagement.Key(ConnectionManagement.Resource.DISPLAY, 1, 1)
        val lease = ConnectionResourceLease(ticket(), key, 1)
        assertFalse(owner.current(lease)); assertFalse(owner.offer(lease, 2, byteArrayOf(1)))
        rejected { owner.read(lease, 2, 1) }; rejected { owner.nativeRetired(lease) }
        rejected { owner.prepared(lease, 0) }; rejected { owner.permissionRequired(lease, 0) }
        rejected { owner.cancel(lease, ConnectionManagement.Reason.REQUESTED) }
        rejected { owner.reject(lease, ConnectionManagement.Outcome.FAILED) }
        owner.tick(0); rejected { owner.tick(-1) }; owner.disconnect(); owner.transportRetired()
    }
    @Test fun t882InvalidWriteCompletionPoisonsPartiallyAdvancedProtocol() {
        val owner = session(); val frame = checkNotNull(owner.nextFrame(0))
        rejected { owner.written(frame, -1) }
        rejected { owner.nextFrame(0) }
        owner.transportRetired()
    }

    @Test fun t882InvitationsPreserveProfileAndOwnedCredentialAcrossWire() {
        val rig = TabletSessionRig()
        for (resource in ConnectionManagement.Resource.entries) {
            val bytes = ByteArray(32) { (it + resource.ordinal).toByte() }
            val invitation = rig.invitation(resource, resource.ordinal + 1L)
                .copy(credential = ConnectionManagement.Credential(bytes))
            rig.send(invitation)
            val event = rig.tablet.nextEvent() as ConnectionSessionEvent.Offered
            assertEquals(invitation.key, event.lease.key)
            assertEquals(invitation.profile, event.profile)
            assertArrayEquals(bytes, event.credential.bytes())
            event.credential.bytes().fill(0)
            assertArrayEquals(bytes, event.credential.bytes())
        }
    }

}
