package com.blent.connection

import org.junit.Assert.*
import org.junit.Test
import java.util.Random

class ConnectionPresentationTest {
    private val computer = ConnectionPeer.parse("ab".repeat(32))
    private val tablet = ConnectionPeer.parse("cd".repeat(32))
    private val ticket = ConnectionTicket(1, computer)
    private fun pending(model: ConnectionPresentation, token: Any): ConnectionChoice {
        model.event(token, ConnectionWorkerEvent.Pairing(ticket, tablet))
        return checkNotNull(model.state.value.approval)
    }

    @Test fun t892CorrespondingLabelsAndExplicitApprovalDoNotPromiseSavedTrust() {
        val model = ConnectionPresentation()
        assertFalse(model.state.value.active)
        assertNull(model.state.value.peer)
        assertNull(model.state.value.remembered)
        val token = model.begin()
        assertTrue(model.state.value.active)
        val choice = pending(model, token)
        assertEquals(computer, choice.ticket.peer)
        assertEquals(tablet, choice.tablet)
        assertFalse(checkNotNull(model.state.value.remembered))
        assertEquals(ConnectionWorkerCommand.Decide(ticket, true, true), model.decide(token, choice, true, true))
        assertNull(model.state.value.approval)
        assertFalse(model.state.value.authenticated)
        assertFalse(checkNotNull(model.state.value.remembered))
        assertNull(model.decide(token, choice, true, true))
        model.event(token, ConnectionWorkerEvent.Trust(computer, true))
        assertTrue(checkNotNull(model.state.value.remembered))
        model.event(token, ConnectionWorkerEvent.State(ConnectionWorkerPhase.AUTHENTICATED))
        assertTrue(model.state.value.authenticated)
    }

    @Test fun t892DenialAndRevocationInvalidateActionsButAwaitDurableTrust() {
        val model = ConnectionPresentation(); val token = model.begin()
        val choice = pending(model, token)
        assertEquals(ConnectionWorkerCommand.Decide(ticket, false, false), model.decide(token, choice, false, true))
        assertFalse(model.state.value.active)
        assertNull(model.revoke(token, choice))
        val next = model.begin()
        model.event(next, ConnectionWorkerEvent.Peer(ticket, tablet, true))
        assertNull(model.state.value.approval)
        val selected = checkNotNull(model.state.value.peer)
        assertEquals(ConnectionWorkerCommand.Revoke(computer), model.revoke(next, selected))
        assertTrue(checkNotNull(model.state.value.remembered))
        assertNull(model.revoke(next, selected))
        model.event(next, ConnectionWorkerEvent.Trust(tablet, false))
        assertTrue(checkNotNull(model.state.value.remembered))
        model.event(next, ConnectionWorkerEvent.Trust(computer, false))
        assertFalse(checkNotNull(model.state.value.remembered))
    }

    @Test fun t892ExactDisplayedChoiceCannotApproveOrRevokeReplacementWithSameTicket() {
        val model = ConnectionPresentation(); val old = model.begin(); val stale = pending(model, old)
        val current = model.begin(); val choice = pending(model, current)
        val random = Random(892)
        repeat(256) {
            val forged = ConnectionChoice(ticket.copy(epoch = random.nextLong()), tablet)
            assertNull(model.decide(current, forged, true, true))
            assertNull(model.revoke(current, forged))
            assertNull(model.decide(old, choice, true, true))
            assertNull(model.revoke(old, choice))
        }
        assertNull(model.decide(current, stale, true, true))
        assertNull(model.revoke(current, stale))
        val unchanged = model.state.value
        model.event(old, ConnectionWorkerEvent.Trust(computer, true))
        model.event(old, ConnectionWorkerEvent.Ended("stale"))
        model.stop(old); model.ended(old, "stale"); model.retired(old); model.started(old)
        assertEquals(unchanged, model.state.value)
        assertNotNull(model.decide(current, choice, true, false))
    }

    @Test fun t892StopRejectsLateMetadataAndReadinessButPreservesTrustUntilRetirement() {
        val model = ConnectionPresentation(); val token = model.begin(); val choice = pending(model, token)
        model.stop(token)
        assertFalse(model.state.value.active); assertNotNull(model.state.value.cleanup)
        assertNull(model.decide(token, choice, true, false)); assertNull(model.revoke(token, choice))
        model.event(token, ConnectionWorkerEvent.Pairing(ticket.copy(epoch = 2), tablet))
        model.event(token, ConnectionWorkerEvent.State(ConnectionWorkerPhase.AUTHENTICATED))
        assertNull(model.state.value.approval); assertFalse(model.state.value.authenticated)
        model.event(token, ConnectionWorkerEvent.Trust(computer, true))
        assertTrue(checkNotNull(model.state.value.remembered))
        model.event(token, ConnectionWorkerEvent.Ended("native failure"))
        assertEquals("native failure", model.state.value.status)
        model.cleanup("native still owned"); assertEquals("native still owned", model.state.value.cleanup)
        model.retired(token); val retired = model.state.value
        assertNull(retired.cleanup)
        model.event(token, ConnectionWorkerEvent.Trust(computer, false))
        model.event(token, ConnectionWorkerEvent.Ended("late"))
        model.stop(token); model.ended(token, "late"); model.started(token)
        assertEquals(retired, model.state.value)
    }

    @Test fun t892PhasesCleanupAndInvalidEpochsStayExplicit() {
        val model = ConnectionPresentation(); val token = model.begin()
        model.cleanup("previous owner retiring")
        model.started(token); assertNull(model.state.value.cleanup)
        for (phase in ConnectionWorkerPhase.entries) {
            model.event(token, ConnectionWorkerEvent.State(phase))
            assertTrue(model.state.value.status.isNotEmpty())
            assertEquals(phase == ConnectionWorkerPhase.AUTHENTICATED, model.state.value.authenticated)
        }
        assertFalse(model.state.value.active)
        model.ended(token, null); assertEquals("USB connection ended.", model.state.value.status)
        val next = model.begin()
        for (epoch in listOf(Long.MIN_VALUE, -1, 0)) {
            assertThrows(IllegalArgumentException::class.java) {
                model.event(next, ConnectionWorkerEvent.Pairing(ticket.copy(epoch = epoch), tablet))
            }
        }
        model.event(next, ConnectionWorkerEvent.Peer(ticket.copy(epoch = Long.MAX_VALUE), tablet, false))
        assertEquals(Long.MAX_VALUE, model.state.value.peer?.ticket?.epoch)
        assertFalse(checkNotNull(model.state.value.remembered))
    }
}
