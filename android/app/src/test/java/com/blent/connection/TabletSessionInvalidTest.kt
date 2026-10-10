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

class TabletSessionInvalidTest {
    @Test fun t882MalformedManagementMutationsCloseConnectionBoundedly() {
        repeat(128) { iteration ->
            val rig = TabletSessionRig(quantum = 7)
            val bytes = ConnectionManagement.encode(rig.invitation())
            bytes[7] = (iteration + 1).toByte() // Reserved byte is never valid.
            assertTrue(rig.host.offer(6, bytes))
            assertThrows(IllegalArgumentException::class.java) { rig.hostDrain() }
            assertThrows(IllegalStateException::class.java) { rig.tablet.nextFrame(0) }
            rig.tablet.transportRetired(); assertTrue(rig.events().isEmpty())
        }
    }
    @Test fun t882UnsupportedHostRecordsAndStaleKeysCannotGrantAuthority() {
        for (operation in 0..3) {
            val rig = TabletSessionRig()
            val lease = rig.offered()
            val message = when (operation) {
                0 -> ConnectionManagement.Capabilities(15)
                1 -> Prepared(lease.key)
                2 -> ResultMessage(lease.key, Outcome.ADMITTED)
                else -> Commit(lease.key.copy(request = 2))
            }
            assertThrows(Exception::class.java) { rig.send(message) }
            assertFalse(rig.tablet.current(lease))
            rig.tablet.nativeRetired(lease); rig.tablet.transportRetired()
            assertTrue(rig.events().contains(ConnectionSessionEvent.State(lease, ConnectionResourceState.RETIRED)))
        }
    }
    @Test fun t882NativeRetirementAndWrongResourceCompletionsAreExact() {
        val rig = TabletSessionRig()
        val lease = rig.admit()
        assertThrows(IllegalStateException::class.java) { rig.tablet.nativeRetired(lease) }
        assertThrows(IllegalArgumentException::class.java) { rig.tablet.read(lease, 3, 1) }
        rig.tablet.cancel(lease, Reason.REQUESTED); rig.tablet.nativeRetired(lease)
        assertThrows(IllegalStateException::class.java) { rig.tablet.nativeRetired(lease) }
        rig.tablet.disconnect(); rig.tablet.transportRetired()
        assertThrows(IllegalStateException::class.java) { rig.tablet.cancel(lease, Reason.REQUESTED) }
    }
    @Test fun t882StalledEventConsumerHasFixedBoundAndCannotGrantAnotherLease() {
        val rig = TabletSessionRig()
        repeat(9) { index -> retireUninstalledWithoutConsumingEvents(rig, index + 1L) }
        rig.send(rig.invitation(request = 10))
        val tenth = ConnectionResourceLease(rig.ticket, rig.invitation(request = 10).key, 10)
        assertThrows(IllegalStateException::class.java) { rig.tablet.prepared(tenth, 0) }
        assertFalse(rig.tablet.current(tenth))
        assertEquals(28, rig.events().size)
        rig.tablet.prepared(tenth, 0); rig.pump()
        assertTrue(rig.messages.contains(Prepared(tenth.key)))
    }
    @Test fun t882FullEventBacklogStillAllowsEveryNativeRetirementAfterDisconnect() {
        val rig = TabletSessionRig()
        repeat(8) { index -> retireUninstalledWithoutConsumingEvents(rig, index + 1L) }
        val leases = Resource.entries.map { resource ->
            val request = resource.ordinal + 9L
            val offer = rig.invitation(resource, request)
            rig.send(offer)
            ConnectionResourceLease(rig.ticket, offer.key, request)
        }
        rig.tablet.disconnect(); rig.tablet.transportRetired()
        for (lease in leases) rig.tablet.nativeRetired(lease)
        val events = rig.events()
        for (lease in leases) assertTrue(events.contains(ConnectionSessionEvent.State(lease, ConnectionResourceState.RETIRED)))
        assertEquals(36, events.size)
    }
    private fun retireUninstalledWithoutConsumingEvents(rig: TabletSessionRig, request: Long) {
        val offer = rig.invitation(request = request)
        rig.send(offer)
        val lease = ConnectionResourceLease(rig.ticket, offer.key, request)
        rig.tablet.reject(lease, Outcome.BUSY); rig.tablet.nativeRetired(lease)
        rig.send(Revoke(lease.key, Reason.REQUESTED)); rig.pump()
    }
}
