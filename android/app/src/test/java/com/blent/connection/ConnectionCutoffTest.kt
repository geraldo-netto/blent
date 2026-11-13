package com.blent.connection

import org.junit.Assert.*
import org.junit.Test

class ConnectionCutoffTest {
    @Test fun t1419AuthorityPrecedesPublicationAndIgnoresMetadataAndRepeatedSnapshots() {
        lateinit var model: ConnectionPresentation
        val before = ArrayList<ConnectionView>()
        model = ConnectionPresentation { before.add(model.state.value) }
        val token = model.begin()
        assertFalse(before.single().active)
        model.cleanup("Retaining original worker")
        model.started(token)
        val route = displayRoute()
        model.event(token, ConnectionWorkerEvent.Peer(route.lease.connection, route.lease.connection.peer, true))
        model.event(token, ConnectionWorkerEvent.Trust(route.lease.connection.peer, true))
        model.event(token, ConnectionWorkerEvent.State(ConnectionWorkerPhase.AUTHENTICATED))
        assertEquals(1, before.size)
        model.event(token, ConnectionWorkerEvent.Display(route))
        assertEquals(2, before.size)
        assertNull(before.last().display)
        val selected = checkNotNull(model.state.value.display)
        model.event(token, ConnectionWorkerEvent.Display(route))
        model.event(token, ConnectionWorkerEvent.State(ConnectionWorkerPhase.AUTHENTICATED))
        assertEquals(2, before.size)
        model.event(token, ConnectionWorkerEvent.DisplayClosed(route.lease))
        assertEquals(3, before.size)
        assertSame(selected, before.last().display)
        model.event(token, ConnectionWorkerEvent.DisplayClosed(route.lease))
        assertEquals(3, before.size)
    }

    @Test fun t1419EndRetirementAndExplicitStopHaveDistinctCutoffsOnlyForActualChanges() {
        var cutoffs = 0
        val model = ConnectionPresentation { cutoffs++ }
        val token = model.begin()
        model.ended(token, "original ended")
        assertEquals(2, cutoffs)
        model.retired(token)
        assertEquals(2, cutoffs)
        model.stop(token)
        assertEquals(3, cutoffs)
        model.stop(token)
        model.cleanup("unchanged ownership")
        assertEquals(3, cutoffs)
        model.begin()
        assertEquals(4, cutoffs)
        model.stop(token)
        assertEquals(4, cutoffs)
    }
}
