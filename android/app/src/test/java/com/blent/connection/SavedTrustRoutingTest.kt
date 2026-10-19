package com.blent.connection

import kotlinx.coroutines.*
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [27, 34])
class SavedTrustRoutingTest {
    private fun page() = ConnectionWorkerEvent.SavedComputers(SavedComputersPage(null, 0, 0, 0, emptyList()))
    @Test fun t1151TrustLaunchUsesOnlyItsOriginalCallbackAndRetiresBeforeUnbind() = runBlocking<Unit> {
        val endpoint = ConnectionWorkerEndpoint().apply { start = { it.emit(page()); it.emit(ConnectionWorkerEvent.Ended(null)) } }
        val context = ConnectionWorkerContext(endpoint)
        val events = mutableListOf<ConnectionWorkerEvent>()
        val worker = AndroidConnectionWorkerRun(context, ConnectionProcessLaunch.Trust(SavedTrustRequest.List), events::add,
            startMillis = 1000, graceMillis = 1, retireMillis = 1000)
        worker.execute(); worker.retire()
        assertEquals(listOf(page().javaClass), events.map { it.javaClass })
        assertTrue(endpoint.codes.contains(ConnectionWorkerIpc.START_TRUST))
        assertFalse(endpoint.codes.contains(ConnectionWorkerIpc.START))
        assertFalse(endpoint.alive); assertEquals(1, context.unbinds)
    }
    @Test fun t1151WrongOperationEventsCannotReachEitherConsumer() = runBlocking<Unit> {
        val wrong = ConnectionWorkerEvent.State(ConnectionWorkerPhase.OPENING)
        val cases = listOf(
            ConnectionProcessLaunch.Trust(SavedTrustRequest.List) to wrong,
            ConnectionProcessLaunch.Connect(ConnectionWorkerSpec("fixture")) to page(),
        )
        for ((launch, event) in cases) {
            val endpoint = ConnectionWorkerEndpoint().apply { start = { it.emit(event) } }
            val events = mutableListOf<ConnectionWorkerEvent>()
            val worker = AndroidConnectionWorkerRun(ConnectionWorkerContext(endpoint), launch, events::add,
                startMillis = 1000, graceMillis = 1, retireMillis = 1000)
            val result = runCatching { worker.execute() }
            worker.retire()
            assertTrue(result.isFailure); assertTrue(events.isEmpty()); assertFalse(endpoint.alive)
        }
    }
    @Test fun t1151StalePageGenerationCannotChangeCurrentResult() = runBlocking<Unit> {
        val endpoint = ConnectionWorkerEndpoint().apply {
            start = {
                it.raw(ConnectionWorkerCodec.event(page()), it.generation + 1)
                it.emit(ConnectionWorkerEvent.Ended(null))
            }
        }
        val events = mutableListOf<ConnectionWorkerEvent>()
        val worker = AndroidConnectionWorkerRun(ConnectionWorkerContext(endpoint), ConnectionProcessLaunch.Trust(SavedTrustRequest.List), events::add,
            startMillis = 1000, graceMillis = 1, retireMillis = 1000)
        worker.execute(); worker.retire()
        assertTrue(events.isEmpty())
    }
}
