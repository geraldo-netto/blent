package com.blent.connection

import java.io.IOException
import kotlinx.coroutines.*
import org.junit.Assert.*

/** Retains one original actor/job and all virtual timer registrations. */
internal class WaitActorRun(val scheduler: ConnectionActorScheduler, val run: ConnectionWorker) {
    private val scope = CoroutineScope(SupervisorJob() + scheduler)
    val execution = scope.async { run.execute() }
    fun outcome(): Exception? = runBlocking {
        check(execution.isCompleted)
        try { execution.await(); null } catch (failure: Exception) { failure }
    }
    fun close(expected: Exception?) {
        try {
            run.cancel(); scheduler.runCurrent()
            assertTrue("T1092 original actor cleanup did not join", execution.isCompleted)
            val failure = outcome()
            assertEquals(expected?.javaClass, failure?.javaClass); assertEquals(expected?.message, failure?.message)
            runBlocking { run.retire() }
        } finally { scope.cancel(); scheduler.runCurrent(); assertTrue(scheduler.empty()) }
    }
}

internal class WorkerWaitRig(val spec: ConnectionWorkerSpec = ConnectionWorkerSpec("fixture", timerMs = 1000), known: Boolean = true) : AutoCloseable {
    val scheduler = ConnectionActorScheduler()
    val state = RuntimeSetup()
    var budget = ConnectionDeadline.IDLE
    var stopRetires = true
    var wake: (() -> Unit)? = null
    private val source = object : ConnectionSetup by state, ConnectionDeadlineSource {
        override fun advance() {
            state.advance()
            val due = budget.atMs
            if (!state.stopping && due != null && scheduler.now >= due) {
                state.error = IOException("T1092 setup deadline"); stop()
            }
        }
        override fun stop() { state.stop(); if (stopRetires) state.gone = true }
        override fun nextDeadline(): ConnectionDeadline = if (state.stopping) ConnectionDeadline.IDLE else budget
    }
    private val provided: ConnectionSetup = if (known) source else object : ConnectionSetup by source {}
    val actor = WaitActorRun(scheduler, AndroidConnectionWorker(spec, { _, signal -> wake = signal; provided }, {}, { scheduler.now }))
    fun start() { scheduler.runCurrent() }
    override fun close() { state.gone = true; wake?.invoke(); actor.close(state.error) }
}

internal class AdmissionWaitRig(val spec: ConnectionWorkerSpec = ConnectionWorkerSpec("fixture", timerMs = 1000), known: Boolean = true) : AutoCloseable {
    val scheduler = ConnectionActorScheduler()
    val state = RunAdmission()
    var budget = ConnectionDeadline.IDLE
    var wake: (() -> Unit)? = null
    private val source = object : AccessoryAdmission by state, ConnectionDeadlineSource {
        override fun advance() {
            state.advance()
            val due = budget.atMs
            if (state.allowed && due != null && scheduler.now >= due) {
                state.error = IOException("T1092 admission deadline"); state.stop()
            }
        }
        override fun nextDeadline(): ConnectionDeadline = if (state.allowed) budget else ConnectionDeadline.IDLE
    }
    private val provided: AccessoryAdmission = if (known) source else object : AccessoryAdmission by source {}
    val actor = WaitActorRun(scheduler, ConnectionAdmissionRun(spec, { signal -> wake = signal; provided },
        { _, _ -> error("T1092 no worker without permission") }, {}, dispatcher = scheduler, clock = { scheduler.now }))
    fun start() { scheduler.runCurrent() }
    override fun close() { state.gone = true; wake?.invoke(); actor.close(state.error) }
}
