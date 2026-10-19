package com.blent.connection

import java.io.IOException
import java.util.concurrent.atomic.AtomicInteger
import kotlinx.coroutines.*
import org.junit.Assert.*
import org.junit.Test

internal class RunAdmission : AccessoryAdmission {
    val selection = AccessorySelection(7, "ab".repeat(32))
    @Volatile var ready: AccessorySelection? = null
    @Volatile var allowed = true
    @Volatile var gone = false
    @Volatile var stopRetires = true
    @Volatile var error: Exception? = null
    @Volatile var terminate = false
    val ticks = AtomicInteger()
    override fun advance() { ticks.incrementAndGet() }
    override fun takeReady() = ready.also { ready = null }
    override fun current(selection: AccessorySelection) = allowed && selection == this.selection
    override fun stop() { allowed = false; if (stopRetires) gone = true }
    override fun retired() = gone
    override fun failure() = error
    override fun terminationRequired() = terminate
}
internal class AdmissionWorker : ConnectionWorker {
    val executing = CompletableDeferred<Unit>()
    val end = CompletableDeferred<Unit>()
    val nativeEnd = CompletableDeferred<Unit>()
    val calls = AtomicInteger()
    val retires = AtomicInteger()
    @Volatile var stopped = false
    @Volatile var stubborn = false
    @Volatile var retireError: Exception? = null
    @Volatile var commandError: Exception? = null
    @Volatile var executeError: Exception? = null
    var terminalAllowed = false
    override fun command(value: ConnectionWorkerCommand): Boolean { commandError?.let { throw it }; calls.incrementAndGet(); return !stopped || (terminalAllowed && (value is ConnectionWorkerCommand.CameraConsumerRetired || value is ConnectionWorkerCommand.AudioConsumerRetired)) }
    override suspend fun execute() {
        executing.complete(Unit)
        if (stubborn) withContext(NonCancellable) { end.await() } else end.await()
        executeError?.let { throw it }
    }
    override fun cancel() { stopped = true; if (!stubborn) end.complete(Unit) }
    override suspend fun retire() { retires.incrementAndGet(); retireError?.let { throw it }; nativeEnd.await() }
}
private class AdmissionRunRig {
    val admission = RunAdmission()
    val worker = AdmissionWorker().apply { nativeEnd.complete(Unit) }
    val made = AtomicInteger()
    val reports = java.util.Collections.synchronizedList(ArrayList<ConnectionWorkerEvent>())
    @Volatile var selected: ConnectionWorkerSpec? = null
    @Volatile var callback: ((ConnectionWorkerEvent) -> Unit)? = null
    @Volatile var wake: (() -> Unit)? = null
    var admissionError: Exception? = null
    var workerError: Exception? = null
    var afterFactory: () -> Unit = {}
    val spec = ConnectionWorkerSpec("unselected", io = ConnectionRunLimits(100, 1), timerMs = 1)
    val run = ConnectionAdmissionRun(spec, { signal ->
        wake = signal; admissionError?.let { throw it }; admission
    }, { spec, report ->
        made.incrementAndGet(); selected = spec; callback = report; workerError?.let { throw it }; afterFactory(); worker
    }, reports::add)
    val peer = ConnectionPeer.parse("cd".repeat(32))
    val command = ConnectionWorkerCommand.Revoke(peer)
    fun grant() { admission.ready = admission.selection; wake?.invoke() }
}

class ConnectionAdmissionRunTest {
    private suspend fun until(condition: () -> Boolean) { withTimeout(3000) { while (!condition()) delay(1) } }
    private suspend fun outcome(action: suspend () -> Unit): Exception? = try { action(); null } catch (failure: Exception) { failure }
    @Test fun t854AdmissionConstructionAndRetirementBeforeExecuteAreInert() = runBlocking {
        val rig = AdmissionRunRig(); rig.run.cancel(); rig.run.retire()
        assertEquals(0, rig.made.get()); assertNull(rig.wake); assertFalse(rig.run.command(rig.command))
        assertTrue(outcome { rig.run.execute() } is IllegalStateException)
    }
    @Test fun t854PermissionPrecedesWorkerAndSelectedTokenReplacesUntrustedTemplate() = runBlocking {
        val rig = AdmissionRunRig(); val executing = async { outcome { rig.run.execute() } }
        until { rig.admission.ticks.get() > 0 }
        assertEquals(0, rig.made.get()); assertFalse(rig.run.command(rig.command))
        rig.grant(); rig.worker.executing.await()
        assertEquals(rig.admission.selection.encode(), checkNotNull(rig.selected).device)
        assertEquals(rig.spec.io, checkNotNull(rig.selected).io)
        assertTrue(rig.run.command(rig.command)); assertEquals(1, rig.worker.calls.get())
        rig.run.cancel(); executing.await(); rig.run.retire()
        assertTrue(rig.admission.gone); assertEquals(1, rig.made.get())
    }
    @Test fun t854DetachGateRejectsCommandAndStalePairingBeforeActorWake() = runBlocking {
        val rig = AdmissionRunRig(); rig.grant(); val executing = async { outcome { rig.run.execute() } }
        rig.worker.executing.await()
        val pairing = ConnectionWorkerEvent.Pairing(ConnectionTicket(1, rig.peer), rig.peer)
        checkNotNull(rig.callback)(pairing); assertEquals(listOf(pairing), rig.reports.toList())
        rig.admission.allowed = false
        assertFalse(rig.run.command(rig.command))
        checkNotNull(rig.callback)(pairing)
        checkNotNull(rig.callback)(ConnectionWorkerEvent.State(ConnectionWorkerPhase.AUTHENTICATED))
        val truth = ConnectionWorkerEvent.Trust(rig.peer, true)
        checkNotNull(rig.callback)(truth)
        val ended = ConnectionWorkerEvent.Ended(null); checkNotNull(rig.callback)(ended)
        executing.await(); rig.run.retire()
        assertEquals(listOf(pairing, truth, ended), rig.reports.toList()); assertTrue(rig.worker.stopped)
    }
    @Test fun t854IndependentTimerFindsAdmissionFailureWhileWorkerIsSuspended() = runBlocking {
        val rig = AdmissionRunRig(); rig.grant(); val executing = async { outcome { rig.run.execute() } }
        rig.worker.executing.await(); rig.admission.error = IOException("Permission observer failed")
        assertTrue(executing.await() is IOException); rig.run.retire(); assertTrue(rig.worker.stopped)
    }
    @Test fun t854FactoryPartialFailureNeverClaimsActualRetirement() = runBlocking {
        val rig = AdmissionRunRig(); rig.admissionError = IOException("Admission factory partial effect")
        assertTrue(outcome { rig.run.execute() } is IOException)
        assertTrue(outcome { rig.run.retire() } is IllegalStateException)
        val second = AdmissionRunRig(); second.workerError = IOException("Binding factory partial effect"); second.grant()
        assertTrue(outcome { second.run.execute() } is IOException)
        assertTrue(outcome { second.run.retire() } is IllegalStateException); assertTrue(second.admission.gone)
    }
    @Test fun t854LateWorkerFactoryResultIsCancelledWithoutExecute() = runBlocking {
        val rig = AdmissionRunRig(); rig.afterFactory = { rig.run.cancel() }; rig.grant()
        assertNull(outcome { rig.run.execute() }); rig.run.retire()
        assertTrue(rig.worker.stopped); assertFalse(rig.worker.executing.isCompleted); assertEquals(1, rig.worker.retires.get())
    }
    @Test fun t854RetirementWaitsBothNativePermissionAndWorkerDeath() = runBlocking {
        val admission = RunAdmission().apply { ready = selection; stopRetires = false }
        val worker = AdmissionWorker()
        val run = ConnectionAdmissionRun(ConnectionWorkerSpec("template", timerMs = 1), { admission }, { _, _ -> worker }, {})
        val executing = async { outcome { run.execute() } }; worker.executing.await(); run.cancel()
        until { worker.retires.get() == 1 }; assertFalse(executing.isCompleted)
        worker.nativeEnd.complete(Unit); delay(5); assertFalse(executing.isCompleted)
        admission.gone = true; executing.await(); run.retire()
    }
    @Test fun t854FailedNativeRetirementCanRetryOnlyAfterActualProof() = runBlocking {
        val rig = AdmissionRunRig(); rig.worker.retireError = IOException("Native worker still alive"); rig.grant()
        val executing = async { outcome { rig.run.execute() } }; rig.worker.executing.await(); rig.run.cancel(); executing.await()
        assertTrue(outcome { rig.run.retire() } is IOException)
        rig.worker.retireError = null; rig.run.retire()
        assertTrue(rig.worker.retires.get() >= 2)
    }
    @Test fun t854CommandFailureCancelsBothOwnedLifetimes() = runBlocking {
        val rig = AdmissionRunRig(); rig.grant(); val executing = async { outcome { rig.run.execute() } }
        rig.worker.executing.await(); rig.worker.commandError = IOException("IPC admission failed")
        assertFalse(rig.run.command(rig.command)); assertTrue(executing.await() is IOException)
        rig.run.retire(); assertTrue(rig.worker.stopped); assertTrue(rig.admission.gone)
    }
    @Test fun t854ReporterFailureCannotLeaveWorkerRunning() = runBlocking {
        val admission = RunAdmission().apply { ready = selection }
        val worker = AdmissionWorker().apply { nativeEnd.complete(Unit) }
        var callback: ((ConnectionWorkerEvent) -> Unit)? = null
        val run = ConnectionAdmissionRun(ConnectionWorkerSpec("template", timerMs = 1), { admission }, { _, report -> callback = report; worker },
            { throw IOException("UI report failed") })
        val executing = async { outcome { run.execute() } }; worker.executing.await()
        checkNotNull(callback)(ConnectionWorkerEvent.State(ConnectionWorkerPhase.AUTHENTICATED))
        assertTrue(executing.await() is IOException); run.retire(); assertTrue(worker.stopped)
    }
    @Test fun t854CleanAdmissionRetirementBeforePermissionEndsRun() = runBlocking {
        val rig = AdmissionRunRig(); rig.admission.gone = true
        withTimeout(100) { rig.run.execute() }
        rig.run.retire(); assertEquals(0, rig.made.get())
    }
    @Test fun t854StoppedActorDoesNotAwaitUncooperativeExecuteBeforeRetirementBudget() = runBlocking {
        val rig = AdmissionRunRig(); rig.worker.stubborn = true; rig.grant()
        val executing = async { outcome { rig.run.execute() } }
        try {
            rig.worker.executing.await(); rig.run.cancel()
            withTimeout(7500) { executing.await() }
            assertFalse("Uncooperative execution remains owned after bounded actor exit", rig.worker.end.isCompleted)
            rig.worker.end.complete(Unit); rig.run.retire()
        } finally { rig.worker.end.complete(Unit); executing.await() }
    }
    @Test fun t909CancellationPublishesExactDisplayRevocationBeforeStuckNativeCleanup() = runBlocking {
        val admission = RunAdmission().apply { ready = selection; stopRetires = false }
        val worker = AdmissionWorker(); val events = java.util.Collections.synchronizedList(ArrayList<ConnectionWorkerEvent>())
        var callback: ((ConnectionWorkerEvent) -> Unit)? = null
        val run = ConnectionAdmissionRun(ConnectionWorkerSpec("template", timerMs = 1), { admission },
            { _, report -> callback = report; worker }, events::add)
        val executing = async { outcome { run.execute() } }
        try {
            worker.executing.await(); val route = displayRoute()
            checkNotNull(callback)(ConnectionWorkerEvent.Display(route))
            run.cancel()
            assertEquals("T909 logical revocation cannot wait for native retirement",
                ConnectionWorkerEvent.DisplayClosed(route.lease), events.last())
            checkNotNull(callback)(ConnectionWorkerEvent.Display(displayRoute(2)))
            assertEquals(2, events.size)
            until { worker.retires.get() > 0 }; assertFalse(executing.isCompleted)
        } finally { worker.nativeEnd.complete(Unit); admission.gone = true; run.cancel(); executing.await(); run.retire() }
    }

    @Test fun t941AdmissionKeepsOnlyExactPublishedCameraCleanupAfterStop() = runBlocking {
        val admission = RunAdmission().apply { ready = selection; stopRetires = false }
        val owned = AdmissionWorker().apply { terminalAllowed = true }; val events = java.util.Collections.synchronizedList(ArrayList<ConnectionWorkerEvent>())
        var callback: ((ConnectionWorkerEvent) -> Unit)? = null
        val run = ConnectionAdmissionRun(ConnectionWorkerSpec("template", timerMs = 1), { admission },
            { _, report -> callback = report; owned }, events::add)
        val execution = async { outcome { run.execute() } }
        try {
            owned.executing.await(); checkNotNull(callback)(ConnectionWorkerEvent.Camera(cameraRoute()))
            run.cancel()
            assertFalse(run.command(ConnectionWorkerCommand.CameraConsumerRetired(cameraLease(2))))
            assertFalse(run.command(ConnectionWorkerCommand.CameraChoice(cameraLease(), CameraConsentChoice(true, true))))
            assertTrue("T941 exact original capture cleanup remains reachable",
                run.command(ConnectionWorkerCommand.CameraConsumerRetired(cameraLease())))
            assertEquals(ConnectionWorkerEvent.CameraClosed(cameraLease()), events.last())
        } finally { owned.nativeEnd.complete(Unit); admission.gone = true; run.cancel(); execution.await(); run.retire() }
    }

    @Test fun t989AudioCleanupRetainsBothOriginalDirectionsAfterAccessoryAuthorityLoss() = runBlocking {
        val rig = AdmissionRunRig(); rig.worker.terminalAllowed = true
        val hold = CompletableDeferred<Unit>()
        val worker = object : ConnectionWorker by rig.worker { override suspend fun retire() { hold.await() } }
        val events = java.util.concurrent.CopyOnWriteArrayList<ConnectionWorkerEvent>()
        var callback: ((ConnectionWorkerEvent) -> Unit)? = null
        val run = ConnectionAdmissionRun(rig.spec, { rig.admission }, { _, report -> callback = report; worker }, events::add)
        rig.grant(); val execution = async { outcome { run.execute() } }; rig.worker.executing.await()
        val resources = listOf(ConnectionManagement.Resource.MICROPHONE, ConnectionManagement.Resource.SPEAKERS)
        try {
            for (resource in resources) {
                val route = audioRoute(resource)
                checkNotNull(callback)(ConnectionWorkerEvent.AudioOffered(LocalAudioOffer(route.lease, route.profile)))
                checkNotNull(callback)(ConnectionWorkerEvent.AudioRoute(route))
                assertFalse(run.command(ConnectionWorkerCommand.AudioConsumerRetired(route.lease)))
            }
            rig.admission.allowed = false
            assertFalse(run.command(ConnectionWorkerCommand.AudioChoice(audioLease(), com.blent.MediaLocalConsent(true, true, true, false))))
            run.cancel()
            assertEquals(2, events.filterIsInstance<ConnectionWorkerEvent.AudioClosed>().size)
            for (resource in resources) {
                assertFalse(run.command(ConnectionWorkerCommand.AudioConsumerRetired(audioLease(resource, 2))))
                assertTrue(run.command(ConnectionWorkerCommand.AudioConsumerRetired(audioLease(resource))))
                assertFalse(run.command(ConnectionWorkerCommand.AudioConsumerRetired(audioLease(resource))))
            }
            assertFalse(execution.isCompleted)
            hold.complete(Unit); assertNull(execution.await()); run.retire()
            assertFalse(run.command(ConnectionWorkerCommand.AudioConsumerRetired(audioLease())))
        } finally { hold.complete(Unit); run.cancel(); execution.await(); run.retire() }
    }
    @Test fun t989RejectedAudioProofIsRetainedForRetryButNeverTransferrableToForeignLease() = runBlocking {
        val rig = AdmissionRunRig(); rig.worker.terminalAllowed = true
        val hold = CompletableDeferred<Unit>()
        val worker = object : ConnectionWorker by rig.worker { override suspend fun retire() { hold.await() } }
        var callback: ((ConnectionWorkerEvent) -> Unit)? = null
        val run = ConnectionAdmissionRun(rig.spec, { rig.admission }, { _, report -> callback = report; worker }, {})
        rig.grant(); val execution = async { outcome { run.execute() } }; rig.worker.executing.await()
        try {
            val route = audioRoute(); checkNotNull(callback)(ConnectionWorkerEvent.AudioRoute(route))
            checkNotNull(callback)(ConnectionWorkerEvent.AudioClosed(route.lease.copy(owner = 2)))
            assertFalse(run.command(ConnectionWorkerCommand.AudioConsumerRetired(route.lease)))
            checkNotNull(callback)(ConnectionWorkerEvent.AudioClosed(route.lease))
            rig.worker.commandError = java.io.IOException("T989 original completion queue failed")
            assertFalse(run.command(ConnectionWorkerCommand.AudioConsumerRetired(route.lease)))
            rig.worker.commandError = null
            assertTrue(run.command(ConnectionWorkerCommand.AudioConsumerRetired(route.lease)))
            hold.complete(Unit); execution.await(); run.retire()
        } finally { hold.complete(Unit); run.cancel(); execution.await(); run.retire() }
    }

}
