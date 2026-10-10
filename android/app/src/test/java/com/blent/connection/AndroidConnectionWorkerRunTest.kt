package com.blent.connection

import android.os.RemoteException
import kotlinx.coroutines.*
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [27, 34])
class AndroidConnectionWorkerRunTest {
    private val spec = ConnectionWorkerSpec("selected-accessory")
    private val peer = ConnectionPeer.parse("ab".repeat(32))
    private fun run(context: ConnectionWorkerContext, report: (ConnectionWorkerEvent) -> Unit = {}, start: Long = 1000, retire: Long = 1000) =
        AndroidConnectionWorkerRun(context, spec, report, bindMillis = 100, startMillis = start, graceMillis = 1, retireMillis = retire)
    private suspend fun until(condition: () -> Boolean) = withTimeout(1500) { while (!condition()) delay(1) }
    private fun await(latch: CountDownLatch) { check(latch.await(3, TimeUnit.SECONDS)) }

    @Test fun t890ConfiguredNativeCleanupMustNotBeKilledAtFixedHalfSecond() = runBlocking {
        val endpoint = ConnectionWorkerEndpoint().apply { start = {} }
        val context = ConnectionWorkerContext(endpoint)
        val configured = spec.copy(io = ConnectionRunLimits(retirementTimeoutMs = 2000))
        val worker = AndroidConnectionWorkerRun(context, configured)
        val executing = async(Dispatchers.Default) { runCatching { worker.execute() } }
        try {
            until { endpoint.callback != null }
            worker.cancel(); executing.await()
            until { endpoint.codes.contains(ConnectionWorkerIpc.STOP) }
            delay(700)
            assertTrue("T890: configured native retirement budget must survive fixed 500 ms EXIT", endpoint.alive)
            assertFalse(endpoint.codes.contains(ConnectionWorkerIpc.EXIT))
        } finally { endpoint.die(); worker.retire() }
    }
    @Test fun t890ExecutionEndedDuringStopAllowsPromptExitButNeverSubstitutesForDeath() = runBlocking {
        val endpoint = ConnectionWorkerEndpoint().apply { start = {}; killOnExit = false }
        val context = ConnectionWorkerContext(endpoint)
        val configured = spec.copy(io = ConnectionRunLimits(retirementTimeoutMs = 60000))
        val worker = AndroidConnectionWorkerRun(context, configured, retireMillis = 100)
        val executing = async(Dispatchers.Default) { runCatching { worker.execute() } }
        try {
            until { endpoint.callback != null }; worker.cancel(); executing.await()
            until { endpoint.codes.contains(ConnectionWorkerIpc.STOP) }
            endpoint.emit(ConnectionWorkerEvent.Ended(null))
            until { endpoint.codes.contains(ConnectionWorkerIpc.EXIT) }
            assertTrue(endpoint.alive)
            assertTrue(runCatching { worker.retire() }.isFailure)
            assertEquals(0, context.unbinds)
        } finally { endpoint.die(); worker.retire() }
    }
    @Test fun t890DefaultDerivedBudgetAcceptsEverySupportedNativeLimit() = runBlocking {
        for (timeout in listOf(1L, 60000L)) {
            val configured = spec.copy(io = ConnectionRunLimits(retirementTimeoutMs = timeout))
            val worker = AndroidConnectionWorkerRun(ConnectionWorkerContext(ConnectionWorkerEndpoint()), configured)
            worker.cancel(); worker.retire()
        }
    }
    @Test fun t884ClientBindIntentMatchesActualServiceGenerationContract() = runBlocking {
        val endpoint = ConnectionWorkerEndpoint()
        val context = ConnectionWorkerContext(endpoint).apply {
            checkIntent = { intent -> assertNotNull(ConnectionWorkerService().onBind(intent)) }
        }
        val worker = run(context)
        try { worker.execute() } finally { worker.retire() }
    }
    @Test fun t884EndedRequiresActualBinderDeathAndExecuteIsSingleAdmission() = runBlocking {
        val endpoint = ConnectionWorkerEndpoint()
        val context = ConnectionWorkerContext(endpoint)
        val run = run(context)
        run.execute()
        assertTrue(runCatching { run.execute() }.isFailure)
        run.retire(); run.retire()
        assertFalse(endpoint.alive); assertEquals(1, context.unbinds); assertTrue(endpoint.deaths.isEmpty())
        assertTrue(endpoint.codes.containsAll(listOf(ConnectionWorkerIpc.START, ConnectionWorkerIpc.STOP, ConnectionWorkerIpc.EXIT)))
        assertFalse(run.command(ConnectionWorkerCommand.Revoke(peer)))
    }
    @Test fun t884CommandsSerializeAndOnlyCurrentWellFormedEventsReachObserver() = runBlocking {
        val endpoint = ConnectionWorkerEndpoint().apply {
            start = {
                assertTrue(it.raw(byteArrayOf(99, 98), it.generation + 1))
                it.emit(ConnectionWorkerEvent.State(ConnectionWorkerPhase.OPENING))
            }
            control = { if (it.controls.size == 2) it.emit(ConnectionWorkerEvent.Ended(null)) }
        }
        val events = mutableListOf<ConnectionWorkerEvent>()
        val run = run(ConnectionWorkerContext(endpoint), events::add)
        val first = ConnectionWorkerCommand.Decide(ConnectionTicket(1, peer), true, false)
        val second = ConnectionWorkerCommand.Revoke(peer)
        assertTrue(run.command(first)); assertTrue(run.command(second))
        run.execute(); run.retire()
        assertEquals(listOf(first, second), endpoint.controls)
        assertEquals(listOf(ConnectionWorkerEvent.State(ConnectionWorkerPhase.OPENING)), events)
    }
    @Test fun t884CommandQueueBoundRejectsWithoutGrowingOrDispatching() = runBlocking {
        val endpoint = ConnectionWorkerEndpoint()
        val run = run(ConnectionWorkerContext(endpoint))
        repeat(32) { assertTrue(run.command(ConnectionWorkerCommand.Revoke(peer))) }
        repeat(128) { assertFalse(run.command(ConnectionWorkerCommand.Revoke(peer))) }
        run.cancel(); run.retire()
        assertTrue(endpoint.codes.isEmpty()); assertTrue(runCatching { run.execute() }.isFailure)
    }
    @Test fun t884BindingDiedCannotSubstituteForLiveBinderDeath() = runBlocking {
        val endpoint = ConnectionWorkerEndpoint().apply { start = {}; killOnExit = false }
        val context = ConnectionWorkerContext(endpoint)
        val run = run(context, retire = 30)
        val active = async(Dispatchers.Default) { runCatching { run.execute() } }
        until { endpoint.callback != null }
        checkNotNull(context.connection).onBindingDied(context.component)
        assertTrue(active.await().isFailure)
        assertTrue(runCatching { run.retire() }.exceptionOrNull() is TimeoutCancellationException)
        assertEquals(0, context.unbinds); assertTrue(endpoint.alive)
        endpoint.die(); run.retire(); assertEquals(1, context.unbinds)
    }
    @Test fun t884FrameworkDisconnectWithoutDeadBinderRemainsQuarantined() = runBlocking {
        val endpoint = ConnectionWorkerEndpoint().apply { start = {}; killOnExit = false }
        val context = ConnectionWorkerContext(endpoint); val run = run(context, retire = 30)
        val active = async(Dispatchers.Default) { runCatching { run.execute() } }
        until { endpoint.callback != null }
        checkNotNull(context.connection).onServiceDisconnected(context.component)
        assertTrue(active.await().isFailure); assertTrue(runCatching { run.retire() }.isFailure)
        endpoint.alive = false // Kernel Binder liveness is also actual death evidence.
        run.retire(); assertEquals(1, context.unbinds)
    }
    @Test fun t884StartSideEffectsBeforeBlockedReturnRemainOwned() = runBlocking {
        val entered = CountDownLatch(1); val release = CountDownLatch(1)
        val endpoint = ConnectionWorkerEndpoint().apply { start = { entered.countDown(); await(release) } }
        val context = ConnectionWorkerContext(endpoint); val run = run(context, start = 30, retire = 30)
        val executing = async(Dispatchers.Default) { runCatching { run.execute() } }
        try {
            withContext(Dispatchers.IO) { await(entered) }
            assertTrue(withTimeout(1000) { executing.await() }.exceptionOrNull() is TimeoutCancellationException)
            until { !endpoint.alive }
            assertTrue(runCatching { run.retire() }.isFailure); assertEquals(0, context.unbinds)
        } finally { release.countDown() }
        run.retire(); assertEquals(1, context.unbinds)
    }
    @Test fun t884BlockedStopCannotPreventIndependentExitOrClaimRetirement() = runBlocking {
        val entered = CountDownLatch(1); val release = CountDownLatch(1)
        val endpoint = ConnectionWorkerEndpoint().apply { stop = { entered.countDown(); await(release) } }
        val context = ConnectionWorkerContext(endpoint); val run = run(context, retire = 30)
        run.execute()
        try {
            withContext(Dispatchers.IO) { await(entered) }
            until { !endpoint.alive }
            assertTrue(runCatching { run.retire() }.isFailure); assertEquals(0, context.unbinds)
        } finally { release.countDown() }
        run.retire(); assertEquals(1, context.unbinds)
    }
    @Test fun t884LateBindingAfterCancelCannotStartNativeWork() = runBlocking {
        val endpoint = ConnectionWorkerEndpoint()
        val context = ConnectionWorkerContext(endpoint).apply { connect = false }
        val run = run(context)
        val execution = async(Dispatchers.Default) { runCatching { run.execute() } }
        until { context.binds == 1 }
        run.cancel(); execution.await(); run.retire()
        checkNotNull(context.connection).onServiceConnected(context.component, endpoint)
        assertFalse(endpoint.codes.contains(ConnectionWorkerIpc.START)); assertEquals(1, context.unbinds)
    }
    @Test fun t884BindRejectionsAndNullBindingsNeverExposeStart() = runBlocking {
        for (mode in 0..2) {
            val endpoint = ConnectionWorkerEndpoint()
            val context = ConnectionWorkerContext(endpoint).apply { accepts = mode != 0; bindFailure = mode == 1; nullBinding = mode == 2 }
            val run = run(context)
            assertTrue(runCatching { run.execute() }.isFailure); run.retire()
            assertFalse(endpoint.codes.contains(ConnectionWorkerIpc.START))
        }
    }
    @Test fun t884LinkFailureDoesNotTurnLiveEndpointIntoDeathProof() = runBlocking {
        val endpoint = ConnectionWorkerEndpoint().apply { failLink = true; killOnExit = false }
        val context = ConnectionWorkerContext(endpoint); val run = run(context, retire = 30)
        assertTrue(runCatching { run.execute() }.isFailure)
        assertTrue(runCatching { run.retire() }.isFailure); assertEquals(0, context.unbinds)
        endpoint.alive = false; run.retire(); assertEquals(1, context.unbinds)
    }
    @Test fun t884EndedFailureAndMalformedEventsCancelWithoutClaimingDeath() = runBlocking {
        for (mode in 0..2) {
            val endpoint = ConnectionWorkerEndpoint().apply { start = {
                when (mode) {
                    0 -> it.emit(ConnectionWorkerEvent.Ended("native failure"))
                    1 -> it.raw(byteArrayOf(99, 98))
                    else -> it.raw(ConnectionWorkerCodec.event(ConnectionWorkerEvent.Ended(null)), trailing = true)
                }
            } }
            val run = run(ConnectionWorkerContext(endpoint))
            assertTrue(runCatching { run.execute() }.isFailure); run.retire()
            assertFalse(endpoint.alive)
        }
    }
    @Test fun t884EventQueueBoundFailsClosedBeforeObserverCanRun() = runBlocking {
        val endpoint = ConnectionWorkerEndpoint().apply { start = { remote ->
            repeat(40) { remote.raw(ConnectionWorkerCodec.event(ConnectionWorkerEvent.State(ConnectionWorkerPhase.OPENING))) }
        } }
        val run = run(ConnectionWorkerContext(endpoint))
        assertTrue(runCatching { run.execute() }.isFailure); run.retire()
    }
    @Test fun t884FailedDeathLinkCannotPublishReadyBeforeRevokingStartAdmission() = runBlocking {
        val endpoint = ConnectionWorkerEndpoint().apply { failLink = true }
        val context = ConnectionWorkerContext(endpoint).apply { connect = false }
        val worker = AndroidConnectionWorkerRun(context, spec, graceMillis = 1,
            transmitDispatcher = Dispatchers.Unconfined)
        val execution = async(start = CoroutineStart.UNDISPATCHED) { runCatching { worker.execute() } }
        checkNotNull(context.connection).onServiceConnected(context.component, endpoint)
        try { execution.await() } finally { worker.retire() }
        assertFalse("Failed death-link must revoke START before publishing readiness", endpoint.codes.contains(ConnectionWorkerIpc.START))
    }
    @Test fun t884LateBinderAfterStopWaitStillReceivesExitAndRetires() = runBlocking {
        val endpoint = ConnectionWorkerEndpoint()
        val context = ConnectionWorkerContext(endpoint).apply { connect = false }
        val worker = AndroidConnectionWorkerRun(context, spec, bindMillis = 10,
            startMillis = 1000, graceMillis = 1, retireMillis = 300)
        assertTrue(runCatching { worker.execute() }.isFailure)
        delay(50) // Both bind wait and initial stop wait have expired.
        checkNotNull(context.connection).onServiceConnected(context.component, endpoint)
        try {
            worker.retire()
            assertTrue(endpoint.codes.contains(ConnectionWorkerIpc.EXIT))
            assertFalse(endpoint.codes.contains(ConnectionWorkerIpc.START))
        } finally { endpoint.die(); worker.retire() }
    }
    @Test fun t884LateBinderDuringUnbindCannotEscapeRetirementDecision() = runBlocking {
        val endpoint = ConnectionWorkerEndpoint()
        val context = ConnectionWorkerContext(endpoint).apply { connect = false }
        val worker = AndroidConnectionWorkerRun(context, spec, bindMillis = 10, graceMillis = 1)
        assertTrue(runCatching { worker.execute() }.isFailure)
        context.duringUnbind = { checkNotNull(context.connection).onServiceConnected(context.component, endpoint) }
        worker.retire()
        assertEquals("Retiring binding must reject late Binder admission", 0, endpoint.links)
        assertFalse(endpoint.codes.contains(ConnectionWorkerIpc.START))
    }
    @Test fun t884ManifestPrivateProcessAndDefaultObserverReceiveNormalState() = runBlocking {
        val endpoint = ConnectionWorkerEndpoint().apply { start = {
            it.emit(ConnectionWorkerEvent.State(ConnectionWorkerPhase.OPENING))
            it.emit(ConnectionWorkerEvent.Ended(null))
        } }
        val context = ConnectionWorkerContext(endpoint)
        val component = android.content.ComponentName(context, ConnectionWorkerService::class.java)
        val info = context.packageManager.getServiceInfo(component, 0)
        assertFalse(info.exported); assertTrue(info.processName.endsWith(":connection"))
        assertEquals(context.applicationInfo.uid, info.applicationInfo.uid)
        val worker = AndroidConnectionWorkerRun(context, spec, graceMillis = 1)
        worker.execute(); worker.retire()
    }
    @Test fun t884GenerationExhaustionAndTimingBoundsNeverWrap() {
        val generations = ConnectionWorkerGenerations(Long.MAX_VALUE - 1)
        assertEquals(Long.MAX_VALUE, generations.next())
        assertThrows(IllegalStateException::class.java) { generations.next() }
        assertThrows(IllegalArgumentException::class.java) { ConnectionWorkerGenerations(-1) }
        val context = ConnectionWorkerContext(ConnectionWorkerEndpoint())
        for (value in listOf(-1L, 60001L)) {
            assertThrows(IllegalArgumentException::class.java) { AndroidConnectionWorkerRun(context, spec, graceMillis = value) }
        }
        for (value in listOf(-1L, 0L, 60001L)) {
            assertThrows(IllegalArgumentException::class.java) { AndroidConnectionWorkerRun(context, spec, bindMillis = value) }
            assertThrows(IllegalArgumentException::class.java) { AndroidConnectionWorkerRun(context, spec, startMillis = value) }
            assertThrows(IllegalArgumentException::class.java) { AndroidConnectionWorkerRun(context, spec, retireMillis = value) }
        }
    }
    @Test fun t909PrivateBinderCarriesDisplayRouteAndExactClosureWithGenerationFiltering() = runBlocking {
        val route = displayRoute()
        val encoded = ConnectionWorkerCodec.event(ConnectionWorkerEvent.Display(route))
        val endpoint = ConnectionWorkerEndpoint().apply {
            start = {
                assertTrue(it.raw(encoded, it.generation + 1))
                it.emit(ConnectionWorkerEvent.Display(route))
                it.emit(ConnectionWorkerEvent.DisplayClosed(route.lease))
                it.emit(ConnectionWorkerEvent.Ended(null))
            }
        }
        val events = ArrayList<ConnectionWorkerEvent>()
        val worker = run(ConnectionWorkerContext(endpoint), events::add)
        try { worker.execute() } finally { worker.retire() }
        assertEquals(2, events.size)
        val decoded = (events[0] as ConnectionWorkerEvent.Display).route
        assertEquals(route.lease, decoded.lease); assertEquals(route.controlPort, decoded.controlPort)
        assertEquals(route.displayPort, decoded.displayPort); assertArrayEquals(route.credential.bytes(), decoded.credential.bytes())
        assertEquals(ConnectionWorkerEvent.DisplayClosed(route.lease), events[1])
    }

}
