package com.blent.connection

import java.util.concurrent.atomic.AtomicBoolean
import java.util.concurrent.atomic.AtomicInteger
import java.util.concurrent.atomic.AtomicReference
import kotlinx.coroutines.*
import kotlinx.coroutines.channels.Channel

/** Main-process composition: USB permission remains live throughout a single
 * isolated connection worker. Detached/stale metadata grants no TLS authority.
 * The retained scope outlives caller cancellation until actual cleanup succeeds. */
internal class ConnectionAdmissionRun(
    private val spec: ConnectionWorkerSpec,
    private val admissionFactory: (wake: () -> Unit) -> AccessoryAdmission,
    private val workerFactory: (ConnectionWorkerSpec, (ConnectionWorkerEvent) -> Unit) -> ConnectionWorker,
    private val report: (ConnectionWorkerEvent) -> Unit,
    dispatcher: CoroutineDispatcher = Dispatchers.Default,
) : ConnectionWorker {
    private val scope = CoroutineScope(SupervisorJob() + dispatcher)
    private val gate = Any()
    private val state = AtomicInteger()
    private val stopping = AtomicBoolean()
    private val error = AtomicReference<Exception?>()
    private val wake = Channel<Unit>(Channel.CONFLATED)
    private val completed = CompletableDeferred<Unit>()
    private var cleanup: Deferred<Unit>? = null
    @Volatile private var admission: AccessoryAdmission? = null
    @Volatile private var selected: AccessorySelection? = null
    @Volatile private var worker: ConnectionWorker? = null
    @Volatile private var execution: Deferred<Unit>? = null
    @Volatile private var uncertain = false
    @Volatile private var retired = false

    private fun signal() { wake.trySend(Unit) }
    private fun current(): Boolean {
        if (stopping.get()) return false
        val selected = selected ?: return false
        return admission?.current(selected) == true
    }
    private fun failed(failure: Exception) { error.compareAndSet(null, failure); cancel() }
    override fun command(value: ConnectionWorkerCommand): Boolean {
        if (!current()) return false
        return try { worker?.command(value) == true }
        catch (failure: Exception) { failed(failure); false }
    }
    private fun deliver(value: ConnectionWorkerEvent) {
        if (!visible(value)) return
        try { report(value) } catch (failure: Exception) { failed(failure) }
    }
    private fun visible(value: ConnectionWorkerEvent): Boolean = when (value) {
        is ConnectionWorkerEvent.Trust, is ConnectionWorkerEvent.Ended -> true
        else -> worker != null && current()
    }
    private fun open() {
        if (stopping.get()) return
        uncertain = true
        admission = admissionFactory(::signal)
        uncertain = false
        if (stopping.get()) admission?.stop()
    }
    private fun select(owned: AccessoryAdmission) {
        if (worker != null || stopping.get()) return
        val selected = owned.takeReady() ?: return
        this.selected = selected
        check(owned.current(selected)) { "Accessory permission selection is no longer current" }
        uncertain = true
        val created = workerFactory(spec.copy(device = selected.encode()), ::deliver)
        worker = created
        uncertain = false
        if (!current()) { cancel(); return }
        execution = scope.async { if (current()) created.execute() }
            .also { it.invokeOnCompletion { signal() } }
    }
    private fun step(owned: AccessoryAdmission): Boolean {
        error.get()?.let { throw it }
        if (stopping.get()) return true
        owned.advance()
        owned.failure()?.let { throw it }
        check(!owned.terminationRequired()) { "Accessory native ownership requires process exit" }
        if (owned.retired()) { cancel(); return true }
        select(owned)
        if (selected != null && !current()) { cancel(); return true }
        val task = execution
        return task?.isCompleted == true
    }
    override suspend fun execute() {
        check(state.compareAndSet(0, 1)) { "Admission already executed or retired" }
        try {
            open()
            val owned = admission ?: return
            while (!step(owned)) waitWake()
            error.get()?.let { throw it }
            val task = execution
            if (task?.isCompleted == true && !stopping.get()) task.await()
        } finally {
            withContext(NonCancellable) {
                this@ConnectionAdmissionRun.cancel()
                try { settle() } catch (_: Exception) { /* Retained for explicit retire/retry. */ }
                state.set(2); completed.complete(Unit)
            }
        }
    }
    override fun cancel() {
        stopping.set(true)
        try { admission?.stop() } catch (failure: Exception) { error.compareAndSet(null, failure) }
        try { worker?.cancel() } catch (failure: Exception) { error.compareAndSet(null, failure) }
        execution?.cancel()
        signal()
    }
    private suspend fun waitWake() { withTimeoutOrNull(spec.timerMs) { wake.receive() } }
    private fun nativeCleanup(): Deferred<Unit> {
        val task = synchronized(gate) {
            val previous = cleanup
            if (previous != null && !previous.isCancelled) return previous
            scope.async<Unit>(start = CoroutineStart.LAZY) { worker?.retire() }.also { cleanup = it }
        }
        task.start()
        return task
    }
    private suspend fun settle() {
        if (retired) return
        check(!uncertain) { "Factory native ownership is unconfirmed" }
        val task = nativeCleanup()
        check(withTimeoutOrNull(spec.io.retirementTimeoutMs + 6000) { awaitRetirement(task); true } == true) {
            "Admission or connection retirement is unconfirmed"
        }
        retired = true
        scope.cancel(); wake.close()
    }
    private suspend fun awaitRetirement(task: Deferred<Unit>) {
        while (true) {
            admission?.advance()
            if (task.isCompleted) {
                task.await()
                if (execution?.isCompleted != false && admission?.retired() != false) return
            }
            waitWake()
        }
    }
    override suspend fun retire() {
        cancel()
        if (state.compareAndSet(0, 2)) completed.complete(Unit)
        check(withTimeoutOrNull(spec.io.retirementTimeoutMs + 6000) { completed.await(); true } == true) {
            "Admission reactor has not completed"
        }
        settle()
    }
}
