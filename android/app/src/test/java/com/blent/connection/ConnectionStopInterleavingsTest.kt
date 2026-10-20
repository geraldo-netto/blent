package com.blent.connection

import com.blent.MediaConsumerClaim
import com.blent.ResourceRetirement
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.runBlocking
import org.junit.Assert.*
import org.junit.Test

/** T1239: exhaustive six-event domain; native proof is supplied explicitly. */
class ConnectionStopInterleavingsTest {
    private val commands = listOf(ConnectionCameraConsumerCommands,
        ConnectionAudioConsumerCommands(ConnectionManagement.Resource.MICROPHONE),
        ConnectionAudioConsumerCommands(ConnectionManagement.Resource.SPEAKERS))
    private val leases = listOf(cameraLease(), audioLease(ConnectionManagement.Resource.MICROPHONE),
        audioLease(ConnectionManagement.Resource.SPEAKERS))

    private class Model {
        val stopped = BooleanArray(3)
        val completed = BooleanArray(3)
        val expected = mutableListOf<ConnectionWorkerCommand>()
    }

    private fun orders(prefix: List<Int>, remaining: Set<Int>): Sequence<List<Int>> = sequence {
        if (remaining.isEmpty()) yield(prefix)
        else for (event in remaining) yieldAll(orders(prefix + event, remaining - event))
    }

    @Test fun t1239EveryStopCompletionOrderingRetainsExactOriginalProof() = runBlocking {
        var traces = 0
        for (order in orders(emptyList(), (0..5).toSet())) {
            verify(order)
            traces++
        }
        assertEquals(720, traces)
    }

    private suspend fun verify(order: List<Int>) {
        val sent = mutableListOf<ConnectionWorkerCommand>()
        val owners = commands.mapIndexed { index, command ->
            ConnectionMediaConsumerOwner(leases[index], command, { sent.add(it); true }, Dispatchers.Unconfined)
        }
        try {
            val claims = owners.map { checkNotNull(it.retain()) }
            val model = Model()
            for (event in order) {
                apply(event, owners, claims, model)
                assertEquals("T1239 trace $order after event $event", model.expected, sent)
                verifyClaims(claims, model)
            }
            assertEquals(6, sent.size)
        } finally { owners.forEach { it.retire() } }
    }

    private fun apply(event: Int, owners: List<ConnectionMediaConsumerOwner>, claims: List<MediaConsumerClaim>, model: Model) {
        val index = event % 3
        if (event < 3) {
            assertTrue(owners[index].stop())
            model.stopped[index] = true
            model.expected.add(commands[index].stop(leases[index]))
            assertFalse(owners[index].closed(leases[index].copy(owner = 2)))
            assertTrue(owners[index].closed(leases[index]))
        } else {
            model.completed[index] = true
            claims[index].retired(ResourceRetirement.RETIRED)
            claims[index].retired(ResourceRetirement.NOT_STARTED)
        }
        if (model.stopped[index] && model.completed[index]) model.expected.add(commands[index].retired(leases[index]))
    }

    private fun verifyClaims(claims: List<MediaConsumerClaim>, model: Model) {
        for (index in claims.indices) {
            assertEquals(!model.stopped[index] && !model.completed[index], claims[index].current())
        }
    }
}
