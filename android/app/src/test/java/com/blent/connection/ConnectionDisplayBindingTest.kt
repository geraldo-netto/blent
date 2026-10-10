package com.blent.connection

import com.blent.*
import org.junit.Assert.*
import org.junit.Test

class ConnectionDisplayBindingTest {
    private fun view(choice: ConnectionDisplayChoice) = ConnectionView(active = true, authenticated = true,
        peer = ConnectionChoice(choice.route.lease.connection, ConnectionPeer.parse("cd".repeat(32))), display = choice)

    @Test fun t907InitialAdbRemainsUntilExplicitUsbSelectionAndExactChoiceChangesAreConflated() {
        val outputs = ArrayList<DisplayEndpoint?>()
        val binding = ConnectionDisplayBinding(DisplayRouteTarget(outputs::add))
        binding.apply(ConnectionView()); assertTrue(outputs.isEmpty())
        binding.apply(ConnectionView(active = true)); assertEquals(listOf<DisplayEndpoint?>(null), outputs)
        val choice = ConnectionDisplayChoice(displayRoute())
        val state = view(choice)
        binding.apply(state); val selected = checkNotNull(outputs.last())
        assertEquals(23456, selected.controlPort); assertEquals(23457, selected.videoPort)
        assertEquals((0 until 32).joinToString("") { "%02x".format(it) }, selected.token)
        binding.apply(state.copy(status = "unchanged route")); assertEquals(2, outputs.size)
        binding.apply(state.copy(display = ConnectionDisplayChoice(displayRoute())))
        assertEquals(3, outputs.size) // Equal counters in a new process still replace.
        binding.apply(ConnectionView()); binding.apply(ConnectionView())
        assertEquals(4, outputs.size); assertNull(outputs.last())
        assertFalse(selected.toString().contains(selected.token))
    }
    @Test fun t907RouteContractRejectsInvalidPortsCredentialsAndSequenceBoundaries() {
        val token = "a".repeat(64)
        val random = java.util.Random(907)
        for (port in listOf(Int.MIN_VALUE, -1, 0, 65536, Int.MAX_VALUE) + List(64) { random.nextInt() or Int.MIN_VALUE }) {
            assertThrows(IllegalArgumentException::class.java) { DisplayEndpoint(token, port, 1) }
            assertThrows(IllegalArgumentException::class.java) { DisplayEndpoint(token, 1, port) }
        }
        assertThrows(IllegalArgumentException::class.java) { DisplayEndpoint(token, 1, 1) }
        for (text in listOf("", "a".repeat(63), "a".repeat(65), "g".repeat(64), "A".repeat(64), "\u0000".repeat(64))) {
            assertThrows(IllegalArgumentException::class.java) { DisplayEndpoint(text, 1, 65535) }
        }
        for (value in listOf(Long.MIN_VALUE, -1, 0)) {
            assertThrows(IllegalArgumentException::class.java) { DisplayRouteSequence(value) }
        }
        val sequence = DisplayRouteSequence(Long.MAX_VALUE - 1)
        assertThrows(IllegalArgumentException::class.java) { sequence.next(token, 0, 1) }
        assertEquals(Long.MAX_VALUE, sequence.next(token, 1, 65535).revision)
        assertThrows(IllegalStateException::class.java) { sequence.next(token, 1, 2) }
    }
    @Test fun t907EveryCredentialByteRoundTripsToExactLowercaseWireToken() {
        val random = java.util.Random(907)
        repeat(256) {
            val bytes = ByteArray(32) { random.nextInt(256).toByte() }
            bytes[0] = Byte.MIN_VALUE; bytes[1] = Byte.MAX_VALUE; bytes[2] = -1; bytes[3] = 0
            var output: DisplayEndpoint? = null
            val binding = ConnectionDisplayBinding(DisplayRouteTarget { output = it })
            val route = displayRoute().copy(credential = ConnectionManagement.Credential(bytes))
            binding.apply(view(ConnectionDisplayChoice(route)))
            assertEquals(bytes.joinToString("") { "%02x".format(it.toInt() and 255) }, checkNotNull(output).token)
        }
    }

    @Test fun t907AdapterCannotGrantRouteFromUnauthenticatedOrForeignPresentationSnapshot() {
        val choice = ConnectionDisplayChoice(displayRoute())
        val validPeer = ConnectionChoice(choice.route.lease.connection, ConnectionPeer.parse("cd".repeat(32)))
        val foreign = ConnectionChoice(validPeer.ticket.copy(epoch = 2), validPeer.tablet)
        val views = listOf(ConnectionView(display = choice), ConnectionView(active = true, display = choice),
            ConnectionView(active = true, authenticated = true, display = choice),
            ConnectionView(active = true, authenticated = true, peer = foreign, display = choice))
        for (view in views) {
            val outputs = ArrayList<DisplayEndpoint?>()
            val binding = ConnectionDisplayBinding(DisplayRouteTarget(outputs::add))
            binding.apply(view)
            assertTrue("T907 malformed snapshot cannot select a native destination", outputs.all { it == null })
        }
    }

}
