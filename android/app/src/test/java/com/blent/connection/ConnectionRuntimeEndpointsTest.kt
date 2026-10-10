package com.blent.connection

import org.junit.Assert.*
import org.junit.Test
import java.util.ArrayDeque

class ConnectionRuntimeEndpointsTest {
    @Test fun t888RealBorrowCleanupAndNativeRetirementRemainPossibleAfterRevoke() {
        val protocol = TabletSessionRig(); val lease = protocol.admit()
        assertTrue(protocol.host.offer(1, byteArrayOf(8, 9))); protocol.pump()
        val rig = RuntimeRig(RuntimeRun(protocol.tablet)); rig.start()
        val endpoints = rig.owner.endpoints(); assertTrue(endpoints.current(lease))
        val receipt = checkNotNull(endpoints.read(lease, 1, 2))
        rig.owner.command(ConnectionWorkerCommand.Revoke(rig.setup.ticket.peer))
        assertFalse(endpoints.current(lease)); assertNull(endpoints.read(lease, 1, 2))
        assertFalse(endpoints.offer(lease, 1, byteArrayOf(3)))
        endpoints.cancel(lease, ConnectionManagement.Reason.REQUESTED)
        endpoints.consumed(lease, 1, receipt)
        assertThrows(IllegalArgumentException::class.java) { endpoints.consumed(lease, 1, receipt) }
        endpoints.nativeRetired(lease); protocol.tablet.transportRetired()
        val states = drain(endpoints).filterIsInstance<ConnectionSessionEvent.State>()
        assertEquals(listOf(ConnectionResourceState.CLOSING, ConnectionResourceState.RETIRED), states.map { it.state })
        assertNull(endpoints.nextEvent())
    }
    private fun drain(endpoints: ConnectionEndpoints): List<ConnectionSessionEvent> {
        val events = ArrayList<ConnectionSessionEvent>()
        repeat(64) { events.add(endpoints.nextEvent() ?: return events) }
        error("T888 endpoint events did not quiesce")
    }
    @Test fun t888PreparedAndPermissionUseFreshClockAndRejectFutureInput() {
        val protocol = TabletSessionRig(); val lease = protocol.offered()
        val observed = ArrayList<Long>()
        val endpoints = object : ConnectionEndpoints by protocol.tablet {
            override fun prepared(lease: ConnectionResourceLease, now: Long) { observed.add(now); protocol.tablet.prepared(lease, now) }
            override fun permissionRequired(lease: ConnectionResourceLease, now: Long) { observed.add(now); protocol.tablet.permissionRequired(lease, now) }
        }
        val rig = RuntimeRig(RuntimeRun(endpoints)); rig.start(); rig.now = 7
        val guarded = rig.owner.endpoints()
        guarded.permissionRequired(lease, 0); guarded.prepared(lease, 1)
        assertEquals(listOf(7L, 7L), observed)
        assertThrows(IllegalArgumentException::class.java) { guarded.prepared(lease, 8) }
        assertThrows(IllegalArgumentException::class.java) { guarded.permissionRequired(lease, -1) }
        rig.owner.stop()
        assertThrows(IllegalStateException::class.java) { guarded.prepared(lease, 7) }
        assertThrows(IllegalStateException::class.java) { guarded.permissionRequired(lease, 7) }
        assertThrows(IllegalStateException::class.java) { guarded.reject(lease, ConnectionManagement.Outcome.BUSY) }
    }
    @Test fun t888CurrentRejectionAndPayloadOwnershipUseMaintainedSession() {
        val protocol = TabletSessionRig(); val lease = protocol.admit()
        val rig = RuntimeRig(RuntimeRun(protocol.tablet)); rig.start()
        val bytes = byteArrayOf(2, 3); assertTrue(rig.owner.endpoints().offer(lease, 1, bytes))
        protocol.pump(); assertArrayEquals(bytes, protocol.payloads.single().second)
        val offered = protocol.offered(ConnectionManagement.Resource.CAMERA, 2)
        rig.owner.endpoints().reject(offered, ConnectionManagement.Outcome.PERMISSION_DENIED)
        protocol.pump()
        assertTrue(protocol.messages.contains(ConnectionManagement.ResultMessage(offered.key, ConnectionManagement.Outcome.PERMISSION_DENIED)))
    }
    @Test fun t888ForeignTicketsCannotUseRetainedCleanupOrNewAdmission() {
        val protocol = TabletSessionRig(); val lease = protocol.admit()
        assertTrue(protocol.host.offer(1, byteArrayOf(1))); protocol.pump()
        val rig = RuntimeRig(RuntimeRun(protocol.tablet)); rig.start(); val endpoints = rig.owner.endpoints()
        val receipt = checkNotNull(endpoints.read(lease, 1, 1)); val stale = lease.copy(connection = lease.connection.copy(epoch = 2))
        assertFalse(endpoints.current(stale)); assertFalse(endpoints.offer(stale, 1, byteArrayOf(1)))
        assertNull(endpoints.read(stale, 1, 1))
        assertThrows(IllegalArgumentException::class.java) { endpoints.cancel(stale, ConnectionManagement.Reason.REQUESTED) }
        assertThrows(IllegalArgumentException::class.java) { endpoints.consumed(stale, 1, receipt) }
        assertThrows(IllegalArgumentException::class.java) { endpoints.nativeRetired(stale) }
        endpoints.consumed(lease, 1, receipt)
        assertTrue(endpoints.current(lease))
    }
    @Test fun t888EventsFilterRevokedAdmissionButPreserveActualClosing() {
        val events = ArrayDeque<ConnectionSessionEvent>()
        val endpoint = object : ConnectionEndpoints by RunnerSession() {
            override fun nextEvent() = events.pollFirst()
        }
        val rig = RuntimeRig(RuntimeRun(endpoint)); rig.start(); val lease = rig.lease()
        val offered = ConnectionSessionEvent.Offered(lease, ConnectionManagement.Credential(ByteArray(32)), ConnectionManagement.Display)
        val readable = ConnectionSessionEvent.Readable(lease, 1)
        val admitted = ConnectionSessionEvent.State(lease, ConnectionResourceState.ADMITTED)
        for (event in listOf(offered, readable, admitted)) {
            events.add(event); assertSame(event, rig.owner.endpoints().nextEvent())
        }
        events.add(readable.copy(lease = lease.copy(connection = lease.connection.copy(epoch = 2))))
        assertNull(rig.owner.endpoints().nextEvent())
        rig.setup.live = false
        repeat(37) { events.add(readable) }
        assertNull(rig.owner.endpoints().nextEvent()); assertEquals(1, events.size)
        events.add(offered); events.add(admitted)
        val closing = ConnectionSessionEvent.State(lease, ConnectionResourceState.CLOSING)
        events.add(closing); assertSame(closing, rig.owner.endpoints().nextEvent())
        assertTrue(rig.run.stopped)
    }
    @Test fun t888ForeignTerminalEventCannotRetireAnotherConnection() {
        val events = ArrayDeque<ConnectionSessionEvent>()
        val endpoint = object : ConnectionEndpoints by RunnerSession() {
            override fun nextEvent() = events.pollFirst()
        }
        val rig = RuntimeRig(RuntimeRun(endpoint)); rig.start()
        val stale = rig.lease(rig.setup.ticket.copy(epoch = 2))
        events.add(ConnectionSessionEvent.State(stale, ConnectionResourceState.CLOSING))
        events.add(ConnectionSessionEvent.State(stale, ConnectionResourceState.RETIRED))
        assertNull(rig.owner.endpoints().nextEvent())
    }
    @Test fun t888SeededStaleTicketsNeverAdmitOrRetireCurrentResource() {
        val protocol = TabletSessionRig(); val lease = protocol.admit()
        val rig = RuntimeRig(RuntimeRun(protocol.tablet)); rig.start()
        val random = java.util.Random(888)
        repeat(256) {
            val stale = lease.copy(connection = lease.connection.copy(epoch = random.nextInt(10000).toLong() + 2))
            val endpoint = rig.owner.endpoints()
            assertFalse(endpoint.current(stale)); assertFalse(endpoint.offer(stale, random.nextInt(), byteArrayOf(1)))
            assertNull(endpoint.read(stale, random.nextInt(), random.nextInt()))
            assertThrows(IllegalArgumentException::class.java) { endpoint.nativeRetired(stale) }
            assertTrue(endpoint.current(lease))
        }
        assertEquals(1, rig.creates); assertFalse(rig.run.stopped)
    }
}
