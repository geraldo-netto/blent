package com.blent.connection

import com.blent.ResourceRetirement
import kotlinx.coroutines.*
import org.junit.Assert.*
import org.junit.Test

class ConnectionCameraSessionTest {
    private fun route(owner: Long = 1) = LocalCameraRoute(cameraLease(owner),
        ConnectionManagement.Credential(ByteArray(32) { it.toByte() }), 23456, cameraProfile())

    @Test fun t948DuplicateDtoKeepsExactTargetButConflictsAndClosedReplaysNeverRevive() = runBlocking {
        val commands = mutableListOf<ConnectionWorkerCommand>()
        val session = ConnectionCameraSession({ commands.add(it); true }, Dispatchers.Unconfined)
        try {
            val route = route(); val target = checkNotNull(session.admitted(route))
            assertSame(target, session.admitted(route.copy(credential = ConnectionManagement.Credential(route.credential.bytes()))))
            assertNull(session.admitted(route.copy(port = 23457)))
            assertNull(session.admitted(route.copy(profile = cameraProfile().copy(fps = 5))))
            assertNull(session.admitted(route.copy(credential = ConnectionManagement.Credential(ByteArray(32)))))
            assertNull(session.admitted(route(2))) // Private resource reuse must wait for original proof.
            val claim = checkNotNull(target.authority.retain())
            session.clear(); assertFalse(claim.current()); assertNull(target.authority.retain())
            assertFalse(session.closed(cameraLease(2)))
            assertTrue(session.closed(route.lease)); assertTrue(commands.isEmpty())
            claim.retired(ResourceRetirement.RETIRED)
            assertEquals(listOf(ConnectionWorkerCommand.CameraConsumerRetired(route.lease)), commands)
            assertNull(session.admitted(route))
            val next = checkNotNull(session.admitted(route(2)))
            assertNotSame(target, next)
            assertNull(session.admitted(route(3).copy(lease = cameraLease(3).copy(connection =
                ConnectionTicket(2, cameraLease().connection.peer)))))
            session.stop(); assertNull(next.authority.retain()); assertNull(session.admitted(route(3)))
        } finally { session.retire() }
    }

    @Test fun t948SameCountersInNewPrivateProcessCannotReceiveOldProof() = runBlocking {
        val oldCommands = mutableListOf<ConnectionWorkerCommand>()
        val newCommands = mutableListOf<ConnectionWorkerCommand>()
        val old = ConnectionCameraSession({ oldCommands.add(it); true }, Dispatchers.Unconfined)
        val next = ConnectionCameraSession({ newCommands.add(it); true }, Dispatchers.Unconfined)
        try {
            val oldTarget = checkNotNull(old.admitted(route()))
            val oldClaim = checkNotNull(oldTarget.authority.retain())
            old.stop()
            val newTarget = checkNotNull(next.admitted(route()))
            val newClaim = checkNotNull(newTarget.authority.retain())
            assertNotSame(oldTarget, newTarget)
            assertTrue(old.closed(cameraLease())); assertTrue(oldCommands.isEmpty())
            oldClaim.retired(ResourceRetirement.NOT_STARTED)
            assertEquals(1, oldCommands.size); assertTrue(newCommands.isEmpty()); assertTrue(newClaim.current())
            next.closed(cameraLease()); assertTrue(newCommands.isEmpty())
            newClaim.retired(ResourceRetirement.RETIRED); assertEquals(1, newCommands.size)
        } finally { old.retire(); next.retire() }
    }

    @Test fun t948ActualWorkerRetirementEndsOriginalRelayAndRefusesFutureMetadata() = runBlocking {
        val session = ConnectionCameraSession({ false }, Dispatchers.Unconfined)
        val target = checkNotNull(session.admitted(route()))
        val claim = checkNotNull(target.authority.retain())
        session.closed(cameraLease()) // Failed admission of terminal proof remains owned.
        session.retire()
        claim.retired(ResourceRetirement.RETIRED)
        assertFalse(claim.current()); assertFalse(target.authority.stop())
        assertNull(session.admitted(route(2))); assertFalse(session.closed(cameraLease()))
        session.retire()
    }
}
