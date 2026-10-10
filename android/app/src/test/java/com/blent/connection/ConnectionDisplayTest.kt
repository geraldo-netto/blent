package com.blent.connection

import java.nio.ByteBuffer
import org.junit.Assert.*
import org.junit.Test

internal fun displayRoute(owner: Long = 1): LocalDisplayRoute = LocalDisplayRoute(
    LocalPumpRig().lease.copy(owner = owner), ConnectionManagement.Credential(ByteArray(32) { it.toByte() }), 23456, 23457)

class ConnectionDisplayTest {
    @Test fun t909PrivateCodecRoundTripsExactLeasePortsAndSecretWithoutFormattingIt() {
        val route = displayRoute()
        val encoded = ConnectionWorkerCodec.event(ConnectionWorkerEvent.Display(route))
        val decoded = (ConnectionWorkerCodec.event(encoded) as ConnectionWorkerEvent.Display).route
        assertEquals(route.lease, decoded.lease); assertEquals(route.controlPort, decoded.controlPort)
        assertEquals(route.displayPort, decoded.displayPort); assertArrayEquals(route.credential.bytes(), decoded.credential.bytes())
        val closed = ConnectionWorkerEvent.DisplayClosed(route.lease)
        assertEquals(closed, ConnectionWorkerCodec.event(ConnectionWorkerCodec.event(closed)))
        assertFalse(route.toString().contains(route.credential.bytes().joinToString()))
        assertTrue(encoded.size < ConnectionWorkerCodec.LIMIT)
    }
    @Test fun t909TruncatedTrailingAndInvalidRouteFieldsRejectBeforePublication() {
        val bytes = ConnectionWorkerCodec.event(ConnectionWorkerEvent.Display(displayRoute()))
        for (length in 0 until bytes.size) {
            assertThrows(Exception::class.java) { ConnectionWorkerCodec.event(bytes.copyOf(length)) }
        }
        assertThrows(Exception::class.java) { ConnectionWorkerCodec.event(bytes + 0) }
        val portOffset = bytes.size - 32 - 8
        for (port in listOf(Int.MIN_VALUE, -1, 0, 65536, Int.MAX_VALUE)) {
            for (offset in listOf(portOffset, portOffset + 4)) {
                val bad = bytes.copyOf(); ByteBuffer.wrap(bad).putInt(offset, port)
                assertThrows(Exception::class.java) { ConnectionWorkerCodec.event(bad) }
            }
        }
        val duplicate = bytes.copyOf(); ByteBuffer.wrap(duplicate).putInt(portOffset, 23457)
        assertThrows(Exception::class.java) { ConnectionWorkerCodec.event(duplicate) }
    }
    @Test fun t909SeededInvalidIpcLeaseCountersRejectAndMaximumDoesNotWrap() {
        val route = displayRoute(); val encoded = ConnectionWorkerCodec.event(ConnectionWorkerEvent.Display(route))
        val leaseEnd = encoded.size - 40
        val random = java.util.Random(909)
        for (value in listOf(Long.MIN_VALUE, -1, 0) + List(128) { random.nextLong() or Long.MIN_VALUE }) {
            for (offset in listOf(2, leaseEnd - 24, leaseEnd - 16, leaseEnd - 8)) {
                val bad = encoded.copyOf(); ByteBuffer.wrap(bad).putLong(offset, value)
                assertThrows(Exception::class.java) { ConnectionWorkerCodec.event(bad) }
            }
        }
        val maximum = ConnectionWorkerEvent.DisplayClosed(route.lease.copy(owner = Long.MAX_VALUE))
        assertEquals(maximum, ConnectionWorkerCodec.event(ConnectionWorkerCodec.event(maximum)))
        assertThrows(IllegalArgumentException::class.java) { ConnectionWorkerEvent.DisplayClosed(route.lease.copy(owner = 0)) }
    }
    @Test fun t909RuntimeSnapshotRequiresCurrentPairingTicketAndResourceAuthority() {
        val endpoints = ResourceEndpoints(); val run = RuntimeRun(endpoints); val rig = RuntimeRig(run)
        run.display = displayRoute(); assertNull(rig.owner.display()); rig.start()
        assertNull(rig.owner.display()); endpoints.active.add(checkNotNull(run.display).lease)
        assertSame(run.display, rig.owner.display())
        run.display = displayRoute().copy(lease = displayRoute().lease.copy(connection = rig.setup.ticket.copy(epoch = 2)))
        endpoints.active.add(checkNotNull(run.display).lease); assertNull(rig.owner.display())
        run.display = displayRoute(); rig.setup.live = false
        assertNull(rig.owner.display()); assertTrue(run.stopped)
        rig.setup.live = true; assertNull(rig.owner.display())
    }
    private fun authenticated(model: ConnectionPresentation, token: Any, route: LocalDisplayRoute) {
        model.event(token, ConnectionWorkerEvent.Peer(route.lease.connection, ConnectionPeer.parse("cd".repeat(32)), true))
        model.event(token, ConnectionWorkerEvent.State(ConnectionWorkerPhase.AUTHENTICATED))
    }
    @Test fun t909PresentationRequiresAuthenticationAndExactProcessGeneration() {
        val model = ConnectionPresentation(); val old = model.begin(); val route = displayRoute()
        model.event(old, ConnectionWorkerEvent.Display(route)); assertNull(model.state.value.display)
        authenticated(model, old, route); model.event(old, ConnectionWorkerEvent.Display(route))
        assertSame(route, model.state.value.display?.route)
        val next = model.begin(); authenticated(model, next, route)
        model.event(old, ConnectionWorkerEvent.Display(route)); assertNull(model.state.value.display)
        model.event(next, ConnectionWorkerEvent.Display(route)); val choice = model.state.value.display
        model.event(next, ConnectionWorkerEvent.Display(route.copy(controlPort = 34567)))
        assertSame(choice, model.state.value.display)
        val foreign = route.copy(lease = route.lease.copy(connection = route.lease.connection.copy(epoch = 2), owner = 2))
        model.event(next, ConnectionWorkerEvent.Display(foreign)); assertSame(choice, model.state.value.display)
        model.event(next, ConnectionWorkerEvent.DisplayClosed(foreign.lease)); assertSame(choice, model.state.value.display)
    }
    @Test fun t909ExactRetirementAndWatermarkRejectDelayedRoutesWithoutClearingReplacement() {
        val model = ConnectionPresentation(); val token = model.begin(); val first = displayRoute()
        authenticated(model, token, first); model.event(token, ConnectionWorkerEvent.Display(first))
        model.event(token, ConnectionWorkerEvent.DisplayClosed(first.lease)); assertNull(model.state.value.display)
        model.event(token, ConnectionWorkerEvent.Display(first)); assertNull(model.state.value.display)
        val next = displayRoute(2); model.event(token, ConnectionWorkerEvent.Display(next))
        val selected = model.state.value.display
        model.event(token, ConnectionWorkerEvent.DisplayClosed(first.lease)); assertSame(selected, model.state.value.display)
        model.event(token, ConnectionWorkerEvent.Display(first)); assertSame(selected, model.state.value.display)
        model.event(token, ConnectionWorkerEvent.DisplayClosed(next.lease.copy(key = next.lease.key.copy(request = 9))))
        assertSame(selected, model.state.value.display)
    }
    @Test fun t909EveryTerminalPathClearsRouteBeforeNativeRetirement() {
        val actions: List<(ConnectionPresentation, Any) -> Unit> = listOf(
            { model, token -> model.stop(token) }, { model, token -> model.ended(token, "worker died") },
            { model, token -> model.revoke(token, checkNotNull(model.state.value.peer)) },
            { model, token -> model.event(token, ConnectionWorkerEvent.State(ConnectionWorkerPhase.CLOSING)) },
            { model, token -> model.retired(token) },
        )
        for (stop in actions) {
            val model = ConnectionPresentation(); val token = model.begin(); val route = displayRoute()
            authenticated(model, token, route); model.event(token, ConnectionWorkerEvent.Display(route))
            stop(model, token); assertNull(model.state.value.display)
            model.event(token, ConnectionWorkerEvent.Display(displayRoute(2))); assertNull(model.state.value.display)
        }
    }
    @Test fun t909RunCompositionExposesOnlyCurrentRoutesAndRevokesOnStop() {
        val rig = ResourceRig(); val transport = ConnectionSessionRunner(RunnerSession(), RunnerIo())
        assertNull(transport.display())
        val run = ConnectionLocalRun(transport, rig.local)
        assertNull(run.display()); rig.offer(); rig.admit(); assertNotNull(run.display())
        run.stop(2); assertNull(run.display())
    }

    @Test fun t909SetupMetadataCannotPublishDisplayWithoutRuntimeAdmission() {
        val rig = RuntimeRig()
        rig.setup.events.add(ConnectionWorkerEvent.Display(displayRoute()))
        rig.setup.events.add(ConnectionWorkerEvent.DisplayClosed(displayRoute().lease))
        assertNull("T909 setup is not a media publication boundary", rig.owner.poll())
        rig.start()
        rig.setup.events.add(ConnectionWorkerEvent.Display(displayRoute()))
        assertNull(rig.owner.poll())
        rig.owner.stop(); rig.setup.events.add(ConnectionWorkerEvent.Display(displayRoute()))
        assertNull(rig.owner.poll())
    }

}
