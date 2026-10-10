package com.blent.connection

import org.junit.Assert.*
import org.junit.Test
import java.io.IOException

class ConnectionRuntimeOwnerTest {
    @Test fun t888PendingApprovalNeverConstructsOrAdmitsAndFacadeIsStable() {
        val rig = RuntimeRig(); val endpoints = rig.owner.endpoints()
        repeat(4) { rig.owner.advance() }
        assertEquals(0, rig.creates); assertSame(endpoints, rig.owner.endpoints())
        assertFalse(endpoints.current(rig.lease())); assertFalse(endpoints.offer(rig.lease(), 1, byteArrayOf(1)))
        assertNull(endpoints.read(rig.lease(), 1, 1)); assertNull(endpoints.nextEvent())
        assertThrows(IllegalStateException::class.java) { endpoints.prepared(rig.lease(), 0) }
        assertThrows(IllegalArgumentException::class.java) { endpoints.nativeRetired(rig.lease()) }
        assertFalse(rig.owner.retired()); assertFalse(rig.owner.terminationRequired()); assertNull(rig.owner.failure())
    }
    @Test fun t888StaleReadyAndDeniedApprovalNeverReachFactory() {
        val rig = RuntimeRig(); rig.setup.offer(); rig.setup.live = false; rig.owner.advance()
        assertEquals(0, rig.creates); assertTrue(rig.setup.stopping)
        rig.setup.offer(); rig.owner.advance(); assertEquals(0, rig.creates)
        assertFalse(rig.owner.command(ConnectionWorkerCommand.Decide(rig.setup.ticket, true, true)))
        rig.setup.gone = true; assertTrue(rig.owner.retired())
    }
    @Test fun t888MatchingRevokeStopsBeforeNextMediaDispatchAtFreshTime() {
        val rig = RuntimeRig(); rig.start(); rig.now = 100
        assertTrue(rig.owner.command(ConnectionWorkerCommand.Revoke(rig.setup.ticket.peer)))
        assertEquals(100L, rig.run.stoppedAt); assertTrue(rig.setup.stopping)
        rig.owner.advance(); assertEquals(listOf(false, true), rig.run.dispatch)
        assertFalse(rig.owner.endpoints().current(rig.lease())); assertEquals(1, rig.creates)
        assertTrue(rig.owner.command(ConnectionWorkerCommand.Revoke(rig.setup.ticket.peer)))
        assertEquals(1, rig.setup.stops)
    }
    @Test fun t888ExternalAuthorityLossIsCheckedBeforeEveryDispatchAndEndpoint() {
        val rig = RuntimeRig(); rig.start(); rig.setup.live = false; rig.now = 5
        assertFalse(rig.owner.endpoints().offer(rig.lease(), 1, byteArrayOf(1)))
        assertEquals(5L, rig.run.stoppedAt); rig.owner.advance(); assertTrue(rig.run.dispatch.last())
        val other = RuntimeRig(); other.start(); other.setup.live = false; other.owner.advance()
        assertTrue(other.run.dispatch.last()); assertTrue(other.setup.stopping)
    }
    @Test fun t888CommandsCorrelateTicketsAndUnrelatedRevokePreservesAuthority() {
        val rig = RuntimeRig()
        assertTrue(rig.owner.command(ConnectionWorkerCommand.Decide(rig.setup.ticket, true, true)))
        assertTrue(rig.setup.decided)
        assertFalse(rig.owner.command(ConnectionWorkerCommand.Decide(rig.setup.ticket.copy(epoch = 2), false, false)))
        rig.start(); val other = ConnectionPeer.parse("cd".repeat(32))
        assertTrue(rig.owner.command(ConnectionWorkerCommand.Revoke(other)))
        assertEquals(other, rig.setup.revoked); assertFalse(rig.run.stopped)
        assertFalse(rig.owner.endpoints().current(rig.lease(rig.setup.ticket.copy(epoch = 2))))
    }
    @Test fun t888FailedFactoryQuarantinesEvenAfterSetupActuallyRetires() {
        val rig = RuntimeRig(); val error = IOException("factory partially constructed")
        rig.factoryError = error; rig.start(); assertSame(error, rig.owner.failure())
        rig.setup.gone = true; rig.setup.offer(); rig.owner.advance()
        assertEquals(1, rig.creates); assertFalse(rig.owner.retired()); assertTrue(rig.owner.terminationRequired())
    }
    @Test fun t888InvalidReadyNeverInvokesFactoryOrLosesNativeOwner() {
        for (capacity in listOf(Int.MIN_VALUE, -1, 0, 4095, 1048577, Int.MAX_VALUE)) {
            val rig = RuntimeRig(); rig.setup.offer(capacity); rig.owner.advance()
            assertEquals(0, rig.creates); assertNotNull(rig.owner.failure()); assertTrue(rig.setup.stopping)
            assertFalse(rig.owner.retired()); rig.setup.gone = true; assertTrue(rig.owner.retired())
        }
        val rig = RuntimeRig(); rig.setup.offer(selected = ConnectionApproval(rig.setup.ticket.copy(epoch = 0), false))
        rig.owner.advance(); assertEquals(0, rig.creates); assertNotNull(rig.owner.failure())
    }
    @Test fun t888BothNativeOwnersMustActuallyRetireAndFailureIsPreserved() {
        val rig = RuntimeRig(); rig.start(); val failure = IOException("I/O failed")
        rig.run.error = failure; rig.owner.advance(); assertSame(failure, rig.owner.failure())
        rig.setup.gone = true; assertFalse(rig.owner.retired())
        rig.run.terminate = true; assertTrue(rig.owner.terminationRequired())
        rig.run.gone = true; assertTrue(rig.owner.retired()); assertTrue(rig.owner.terminationRequired())
        val setup = RuntimeRig(); setup.setup.error = failure; setup.owner.advance()
        assertSame(failure, setup.owner.failure()); assertTrue(setup.setup.stopping)
        setup.setup.terminate = true; assertTrue(setup.owner.terminationRequired())
    }
    @Test fun t888StopIsPromptIdempotentAndThrownStopQuarantines() {
        val rig = RuntimeRig(); rig.start(); rig.run.stopError = IOException("cannot prove stop")
        rig.owner.stop(); rig.owner.stop(); assertEquals(1, rig.setup.stops)
        assertTrue(rig.owner.terminationRequired()); assertNotNull(rig.owner.failure())
        rig.run.gone = true; rig.setup.gone = true; assertFalse(rig.owner.retired())
    }
    @Test fun t888InvalidAndBackwardClockClosesUsingLastValidTime() {
        for (bad in listOf(Long.MIN_VALUE, -1, 19, Long.MAX_VALUE - 59999, Long.MAX_VALUE)) {
            val rig = RuntimeRig(); rig.now = 20; rig.start(); rig.now = bad; rig.owner.advance()
            assertEquals(20L, rig.run.stoppedAt); assertNotNull(rig.owner.failure()); assertTrue(rig.setup.stopping)
        }
        val rig = RuntimeRig(); rig.start(); rig.clockError = IOException("clock failed"); rig.owner.stop()
        assertNotNull(rig.owner.failure()); assertTrue(rig.setup.stopping)
    }
    @Test fun t888CleanSetupRetirementBeforeReadyStopsWithoutFactoryAdmission() {
        val rig = RuntimeRig(); rig.setup.gone = true; rig.owner.advance()
        assertTrue(rig.setup.stopping); assertTrue(rig.owner.retired()); assertEquals(0, rig.creates)
    }
    @Test fun t888EndedNotificationStartsCleanupBeforeActualSetupRetirement() {
        val rig = RuntimeRig(); val ended = ConnectionWorkerEvent.Ended(null)
        rig.setup.events.add(ended); assertSame(ended, rig.owner.poll())
        assertTrue(rig.setup.stopping); assertFalse(rig.owner.retired())
        rig.setup.gone = true; assertTrue(rig.owner.retired())
    }
    @Test fun t888CleanRunnerRetirementReleasesRetainedSetup() {
        val rig = RuntimeRig(); rig.start(); rig.run.gone = true; rig.owner.advance()
        assertTrue(rig.setup.stopping); assertFalse(rig.owner.retired())
        rig.setup.gone = true; assertTrue(rig.owner.retired()); assertNull(rig.owner.failure())
    }
    @Test fun t895PeerMetadataCannotEscapeAfterLogicalStop() {
        val rig = RuntimeRig()
        val peer = ConnectionWorkerEvent.Peer(rig.setup.ticket, rig.setup.ticket.peer, true)
        rig.setup.events.add(peer); assertSame(peer, rig.owner.poll())
        rig.owner.stop()
        rig.setup.events.add(peer); assertNull("T895 stale identity is not a durable trust event", rig.owner.poll())
    }
    @Test fun t888EventFilteringIsBoundedAndPreservesDurableTrustAfterStop() {
        val rig = RuntimeRig()
        val pairing = ConnectionWorkerEvent.Pairing(rig.setup.ticket, rig.setup.ticket.peer)
        rig.setup.events.add(pairing); assertSame(pairing, rig.owner.poll())
        rig.setup.events.add(ConnectionWorkerEvent.State(ConnectionWorkerPhase.OPENING)); assertNotNull(rig.owner.poll())
        rig.owner.stop()
        repeat(33) { rig.setup.events.add(pairing) }
        assertNull(rig.owner.poll()); assertEquals(1, rig.setup.events.size)
        rig.setup.events.add(ConnectionWorkerEvent.State(ConnectionWorkerPhase.AUTHENTICATED))
        val trust = ConnectionWorkerEvent.Trust(rig.setup.ticket.peer, true)
        rig.setup.events.add(trust); assertSame(trust, rig.owner.poll())
        val closing = ConnectionWorkerEvent.State(ConnectionWorkerPhase.CLOSING)
        rig.setup.events.add(closing); assertSame(closing, rig.owner.poll())
        val ended = ConnectionWorkerEvent.Ended(null); rig.setup.events.add(ended)
        assertSame(ended, rig.owner.poll()); assertNull(rig.owner.poll())
    }
}
