package com.blent.connection

import java.io.IOException
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicLong
import kotlinx.coroutines.*
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [27, 34])
class AndroidConnectionWorkerTest {
    private fun spec() = ConnectionWorkerSpec("1:" + "ab".repeat(32),
        session = ConnectionSessionLimits(capacity = 4096), io = ConnectionRunLimits(100, 50), timerMs = 1)
    private suspend fun until(condition: () -> Boolean) { withTimeout(3000) { while (!condition()) delay(1) } }
    private suspend fun failed(action: suspend () -> Unit): Exception {
        try { action() } catch (failure: Exception) { return failure }
        error("Expected failure")
    }
    private class Rig {
        val setup = RuntimeSetup()
        val endpoints = ResourceEndpoints()
        val run = RuntimeRun(endpoints)
        val reports = ArrayList<ConnectionWorkerEvent>()
        var creates = 0
        var live: (() -> Boolean)? = null
        var wake: (() -> Unit)? = null
        var setupError: Exception? = null
        var retire = true
        var time = 0L
        fun create(spec: ConnectionWorkerSpec): AndroidConnectionWorker = AndroidConnectionWorker(spec, { current, signal ->
            creates++; live = current; wake = signal; setupError?.let { throw it }
            object : ConnectionSetup by setup {
                override fun stop() { setup.stop(); if (retire) setup.gone = true }
            }
        }, reports::add, { time }, ConnectionRunFactory {
            object : ConnectionRun by run {
                override fun stop(now: Long) { run.stop(now); if (retire) run.gone = true }
            }
        })
    }
    @Test fun t854FactoryIsPromptAndCancelBeforeExecuteOpensNothing() = runBlocking {
        val rig = Rig(); val worker = rig.create(spec())
        assertEquals(0, rig.creates); worker.cancel(); worker.execute(); worker.retire()
        assertEquals(0, rig.creates); assertFalse(worker.command(ConnectionWorkerCommand.Revoke(rig.setup.ticket.peer)))
        assertTrue(failed { worker.execute() } is IllegalStateException)
    }
    @Test fun t854RetireBeforeExecutePreventsLaterNativeAdmission() = runBlocking {
        val rig = Rig(); val worker = rig.create(spec()); worker.retire(); worker.retire()
        assertTrue(failed { worker.execute() } is IllegalStateException); assertEquals(0, rig.creates)
    }
    @Test fun t854OneActorBoundsCommandsAndSuppressesInternalEnded() = runBlocking {
        val rig = Rig(); val worker = rig.create(spec())
        val command = ConnectionWorkerCommand.Decide(rig.setup.ticket, true, true)
        repeat(32) { assertTrue(worker.command(command)) }; assertFalse(worker.command(command))
        val execution = async { worker.execute() }
        until { rig.setup.decided }; assertEquals(1, rig.creates); assertTrue(checkNotNull(rig.live)())
        val trust = ConnectionWorkerEvent.Trust(rig.setup.ticket.peer, true)
        rig.setup.events.add(trust); rig.setup.events.add(ConnectionWorkerEvent.Ended(null)); checkNotNull(rig.wake)()
        execution.await(); worker.retire()
        assertEquals(listOf(trust), rig.reports); assertFalse(checkNotNull(rig.live)())
    }
    @Test fun t854CancelInvalidatesOpeningGateAndStopsRunnerBeforeFurtherDispatch() = runBlocking {
        val rig = Rig(); rig.setup.offer(); val worker = rig.create(spec())
        val execution = async { worker.execute() }; until { rig.run.dispatch.isNotEmpty() }
        worker.cancel(); assertFalse(checkNotNull(rig.live)())
        execution.await(); worker.retire()
        assertTrue(rig.run.stopped); assertTrue(rig.setup.stopping); assertTrue(rig.run.dispatch.last())
    }
    @Test fun t854IndependentTimerObservesSetupFailureWithoutWake() = runBlocking {
        supervisorScope {
            val rig = Rig(); val worker = rig.create(spec()); val execution = async { worker.execute() }
            until { rig.creates == 1 }; val failure = IOException("Native operation failed")
            rig.setup.error = failure
            val observed = failed { execution.await() }
            assertEquals(failure.javaClass, observed.javaClass); assertEquals(failure.message, observed.message); worker.retire()
            assertTrue(rig.setup.stopping)
        }
    }
    @Test fun t854SetupFactoryFailureQuarantinesRatherThanRetrying() = runBlocking {
        val rig = Rig(); rig.setupError = IOException("Partial setup ownership")
        val worker = rig.create(spec())
        assertSame(rig.setupError, failed { worker.execute() })
        assertTrue(failed { worker.retire() } is IllegalStateException); assertEquals(1, rig.creates)
    }
    @Test fun t854CoroutineCancellationStillWaitsActualNativeRetirement() = runBlocking {
        val rig = Rig(); rig.retire = false; val worker = rig.create(spec())
        val execution = launch { worker.execute() }; until { rig.creates == 1 }
        execution.cancel(); until { rig.setup.stopping }
        assertFalse(execution.isCompleted)
        rig.setup.gone = true; checkNotNull(rig.wake)(); execution.join(); worker.retire()
    }
    @Test fun t854StuckNativeRetirementIsBoundedAndNeverReportedAsSuccess() = runBlocking {
        val rig = Rig(); rig.retire = false; val worker = rig.create(spec())
        val execution = async { worker.execute() }; until { rig.creates == 1 }
        worker.cancel(); rig.setup.events.add(ConnectionWorkerEvent.Ended(null)); checkNotNull(rig.wake)()
        withTimeout(3000) { execution.await() }
        assertTrue(failed { worker.retire() } is IllegalStateException)
        rig.setup.gone = true
        assertTrue("A timed-out actor remains quarantined", failed { worker.retire() } is IllegalStateException)
    }
    @Test fun t854ProcessExitRequirementFailsExecutionEvenWithNoProtocolError() = runBlocking {
        supervisorScope {
            val rig = Rig(); val worker = rig.create(spec()); val execution = async { worker.execute() }
            until { rig.creates == 1 }; rig.setup.terminate = true
            assertTrue(failed { execution.await() } is IllegalStateException)
            assertTrue(failed { worker.retire() } is IllegalStateException)
        }
    }
    @Test fun t854ReportFailureCannotOrphanSetupAndDuplicateExecuteRejects() = runBlocking {
        supervisorScope {
            val setup = RuntimeSetup(); var made = false
            val worker = AndroidConnectionWorker(spec(), { _, _ -> made = true; object : ConnectionSetup by setup {
                override fun stop() { setup.stop(); setup.gone = true }
            } }, { throw IOException("Reporter failed") }, { 0 })
            val execution = async { worker.execute() }; until { made }
            assertTrue(failed { worker.execute() } is IllegalStateException)
            setup.events.add(ConnectionWorkerEvent.State(ConnectionWorkerPhase.OPENING))
            assertTrue(failed { execution.await() } is IOException); worker.retire(); assertTrue(setup.stopping)
        }
    }
    @Test fun t854DefaultRunnerAndAndroidClockOwnTransportUntilActualExit() = runBlocking {
        val setup = RuntimeSetup(); val io = RunnerIo()
        setup.ready = ConnectionSetupReady(io, setup.approval, 4096)
        val worker = AndroidConnectionWorker(spec(), { _, _ -> object : ConnectionSetup by setup {
            override fun stop() { setup.stop(); setup.gone = true; io.gone = true }
        } }, {})
        val execution = async { worker.execute() }; until { io.requests.isNotEmpty() }
        assertEquals(2, io.requests.size); worker.cancel(); execution.await(); worker.retire()
        assertTrue(io.stopping); assertTrue(setup.stopping)
    }
    @Test fun t854NativeSetupStallDoesNotBlockTimerOrCancellation() = runBlocking {
        supervisorScope {
            val native = SetupBackend().apply { discoverGate = CountDownLatch(1) }
            val time = AtomicLong(10); val store = SetupStore()
            val worker = AndroidConnectionWorker(spec(), { _, wake -> ConnectionSetupOwner(native, { store }, time::get,
                ConnectionSetupLimits(4096, 1000, 100, 50), wake) }, {}, time::get)
            val execution = async { worker.execute() }
            try {
                until { native.discoverEntered.count == 0L }; time.set(110)
                assertTrue(failed { execution.await() } is IllegalStateException)
                assertTrue(native.closed); assertTrue(failed { worker.retire() } is IllegalStateException)
            } finally { native.discoverGate.countDown() }
            until { store.closed }
        }
    }
    @Test fun t909WorkerPublishesBoundedRouteTransitionsAndExactRevocation() = runBlocking {
        val rig = Rig(); rig.setup.offer(); val first = displayRoute()
        rig.run.display = first; rig.endpoints.active.add(first.lease)
        val worker = rig.create(spec()); val execution = async { worker.execute() }
        try {
            until { rig.reports.any { it is ConnectionWorkerEvent.Display } }
            delay(5); assertEquals(listOf(ConnectionWorkerEvent.Display(first)), rig.reports)
            val next = displayRoute(2); rig.run.display = next; rig.endpoints.active.add(next.lease)
            checkNotNull(rig.wake)(); until { rig.reports.size == 3 }
            assertEquals(ConnectionWorkerEvent.DisplayClosed(first.lease), rig.reports[1])
            assertEquals(ConnectionWorkerEvent.Display(next), rig.reports[2])
            rig.endpoints.active.clear(); checkNotNull(rig.wake)(); until { rig.reports.size == 4 }
            assertEquals(ConnectionWorkerEvent.DisplayClosed(next.lease), rig.reports[3]); assertFalse(rig.run.stopped)
        } finally { worker.cancel(); execution.await(); worker.retire() }
    }
    @Test fun t909PairingLossPublishesDisplayRevocationDuringWorkerCleanup() = runBlocking {
        val rig = Rig(); rig.setup.offer(); val route = displayRoute()
        rig.run.display = route; rig.endpoints.active.add(route.lease)
        val worker = rig.create(spec()); val execution = async { worker.execute() }
        try {
            until { rig.reports.isNotEmpty() }; rig.setup.live = false; checkNotNull(rig.wake)()
            execution.await(); worker.retire()
            assertEquals(listOf(ConnectionWorkerEvent.Display(route), ConnectionWorkerEvent.DisplayClosed(route.lease)), rig.reports)
        } finally { worker.cancel(); execution.await() }
    }

}
