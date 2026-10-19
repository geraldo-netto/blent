package com.blent.connection

import android.os.SystemClock
import java.util.concurrent.atomic.AtomicBoolean
import java.util.concurrent.atomic.AtomicInteger

import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.NonCancellable
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.withContext
import kotlinx.coroutines.withTimeoutOrNull

/** Authenticated setup supplies the ticket/window; media consent remains owned
 * by its resource controller. No media device opens during composition. */
internal fun tabletConnectionRun(spec: ConnectionWorkerSpec, ready: ConnectionSetupReady, wake: () -> Unit): ConnectionRun {
    val session = TabletSession(ready.approval.ticket, spec.session.copy(capacity = ready.capacity), 15)
    return ConnectionLocalRun(ConnectionSessionRunner(session, ready.io, spec.io),
        ConnectionLocalResources(session, ready.capacity, spec.io, cameraAuthority = session, audioAuthority = session, microphoneAuthority = session, wake = wake))
}

/** One connection-process actor. Native opening, crypto and storage belong to
 * setup's retained workers; callbacks only wake this serialized control loop. */
internal class AndroidConnectionWorker(
    private val spec: ConnectionWorkerSpec,
    private val setup: (current: () -> Boolean, wake: () -> Unit) -> ConnectionSetup,
    private val report: (ConnectionWorkerEvent) -> Unit,
    private val clock: () -> Long = SystemClock::elapsedRealtime,
    runner: ConnectionRunFactory? = null,
) : ConnectionWorker {
    private val state = AtomicInteger(0)
    private val stopping = AtomicBoolean()
    private val wake = Channel<Unit>(Channel.CONFLATED)
    private val runner = runner ?: ConnectionRunFactory { ready -> tabletConnectionRun(spec, ready, ::signal) }
    private val commands = Channel<ConnectionWorkerCommand>(32)
    private val completed = CompletableDeferred<Unit>()
    private var displayed: LocalDisplayRoute? = null
    private val camera = ConnectionCameraPublication(report)
    private val microphone = ConnectionAudioPublication(report)
    private val speakers = ConnectionAudioPublication(report)
    @Volatile private var runtime: ConnectionRuntime? = null
    @Volatile private var uncertain = false
    @Volatile private var retirementError: Exception? = null

    private fun signal() { wake.trySend(Unit) }
    private fun current(): Boolean = !stopping.get()
    override fun command(value: ConnectionWorkerCommand): Boolean =
        (current() || terminal(value)) && state.get() != 2 &&
            commands.trySend(value).isSuccess.also { if (it) signal() }
    private fun terminal(value: ConnectionWorkerCommand): Boolean =
        value is ConnectionWorkerCommand.CameraConsumerRetired || value is ConnectionWorkerCommand.AudioConsumerRetired
    override fun cancel() { stopping.set(true); signal() }

    private fun create() {
        if (!current()) return
        uncertain = true
        val authority = setup(::current, ::signal)
        runtime = ConnectionRuntimeOwner(authority, runner, clock)
        uncertain = false
        if (!current()) checkNotNull(runtime).stop()
    }
    override suspend fun execute() {
        check(state.compareAndSet(0, 1)) { "Connection worker already executed or retired" }
        try {
            create()
            val owned = runtime ?: return
            while (!step(owned)) waitWake()
        } finally {
            withContext(NonCancellable) { finish() }
        }
    }
    private fun controls(owned: ConnectionRuntime) {
        repeat(32) {
            val command = commands.tryReceive().getOrNull() ?: return
            if (current() || terminal(command)) owned.command(command)
        }
    }
    private fun events(owned: ConnectionRuntime) {
        repeat(32) {
            val event = owned.poll() ?: return
            // Service publishes execution Ended after this worker's retirement.
            if (event !is ConnectionWorkerEvent.Ended) report(event)
        }
    }
    private fun step(owned: ConnectionRuntime): Boolean {
        if (!current()) { owned.stop(); return true }
        controls(owned)
        owned.advance()
        events(owned)
        display(owned)
        camera(owned)
        audio(owned)
        owned.failure()?.let { throw it }
        check(!owned.terminationRequired()) { "Connection native ownership requires process exit" }
        return owned.retired()
    }
    private fun display(owned: ConnectionRuntime) {
        val selected = owned.display()
        if (selected == displayed) return
        displayed?.let { report(ConnectionWorkerEvent.DisplayClosed(it.lease)) }
        displayed = selected
        selected?.let { report(ConnectionWorkerEvent.Display(it)) }
    }
    private fun camera(owned: ConnectionRuntime) {
        val projection = owned as? LocalCameraProjection
        camera.update(projection?.offered(), projection?.camera())
    }
    private fun audio(owned: ConnectionRuntime) {
        val projection = owned as? LocalAudioProjection
        microphone.update(projection?.audioOffered(ConnectionManagement.Resource.MICROPHONE), projection?.audio(ConnectionManagement.Resource.MICROPHONE))
        speakers.update(projection?.audioOffered(ConnectionManagement.Resource.SPEAKERS), projection?.audio(ConnectionManagement.Resource.SPEAKERS))
    }
    private suspend fun waitWake() { withTimeoutOrNull(spec.timerMs) { wake.receive() } }
    private suspend fun cleanup(owned: ConnectionRuntime): Boolean {
        owned.stop()
        val result = withTimeoutOrNull(spec.io.retirementTimeoutMs) {
            while (true) {
                controls(owned); owned.advance(); events(owned); display(owned); camera(owned); audio(owned)
                if (owned.terminationRequired()) return@withTimeoutOrNull false
                if (owned.retired()) return@withTimeoutOrNull true
                waitWake()
            }
            @Suppress("UNREACHABLE_CODE") false
        }
        return result == true
    }
    private suspend fun finish() {
        cancel()
        try {
            val owned = runtime
            check(!uncertain && (owned == null || cleanup(owned))) { "Connection native retirement is unconfirmed" }
        } catch (failure: Exception) { retirementError = failure }
        finally {
            state.set(2); commands.close(); wake.close(); completed.complete(Unit)
        }
    }
    override suspend fun retire() {
        cancel()
        if (state.compareAndSet(0, 2)) completed.complete(Unit)
        check(withTimeoutOrNull(spec.io.retirementTimeoutMs + spec.timerMs) { completed.await(); true } == true) {
            "Connection actor has not retired"
        }
        retirementError?.let { throw it }
    }
}
