package com.blent.connection

import android.content.ComponentName
import android.content.Context
import android.content.Intent
import android.content.ServiceConnection
import android.os.Binder
import android.os.IBinder
import android.os.Parcel
import kotlinx.coroutines.*
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.selects.select
import java.io.IOException
import java.util.concurrent.atomic.AtomicBoolean
import java.util.concurrent.atomic.AtomicLong
import java.util.concurrent.atomic.AtomicReference

/** Process-local callback generation; exhaustion is terminal, never wrapping. */
internal class ConnectionWorkerGenerations(initial: Long = 0) {
    private val value = AtomicLong(initial)
    init { require(initial >= 0) }
    fun next(): Long {
        while (true) {
            val previous = value.get()
            check(previous < Long.MAX_VALUE) { "Connection worker generations exhausted" }
            if (value.compareAndSet(previous, previous + 1)) return previous + 1
        }
    }
}

/** Main-process adapter. Native IPC jobs retain ownership independently of their
 * awaiting caller. Ended/framework disconnect never substitutes for Binder death.
 * Failed retirement retains the binding and jobs; a later retry can observe exit. */
internal class AndroidConnectionWorkerRun(
    context: Context,
    private val spec: ConnectionWorkerSpec,
    private val report: (ConnectionWorkerEvent) -> Unit = {},
    private val bindMillis: Long = 5000,
    private val startMillis: Long = 5000,
    graceMillis: Long? = null,
    retireMillis: Long? = null,
    generations: ConnectionWorkerGenerations = sharedGenerations,
    private val transmitDispatcher: CoroutineDispatcher = Dispatchers.IO,
) : ConnectionWorker {
    companion object { private val sharedGenerations = ConnectionWorkerGenerations() }
    // Native retirement plus the maximum supported actor timer cadence. Explicit
    // overrides remain bounded to 60 seconds. Ended permits EXIT, never retirement.
    private val grace = graceMillis ?: (spec.io.retirementTimeoutMs + 1000)
    private val retirementWait = retireMillis ?: (grace + 5000)
    private val app = context.applicationContext
    private val generation = generations.next()
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Default)
    private val guard = Any()
    private val admitted = AtomicBoolean()
    private val stopped = AtomicBoolean()
    private val failure = AtomicReference<Exception?>()
    private val ready = CompletableDeferred<IBinder>()
    private val started = CompletableDeferred<Unit>()
    private val dead = CompletableDeferred<Unit>()
    private val ended = CompletableDeferred<Unit>()
    private val commands = Channel<ByteArray>(32)
    private val events = Channel<ByteArray>(32)
    @Volatile private var bound = false
    @Volatile private var remote: IBinder? = null
    @Volatile private var startExposed = false
    private var acceptingBinding = true
    private var outgoing: Job? = null
    private var stopping: Job? = null
    private var stopCall: Job? = null
    private var exitCall: Job? = null
    private var cleanup: Deferred<Unit>? = null
    @Volatile private var retired = false
    init {
        require(bindMillis in 1..60000 && startMillis in 1..60000)
        require(graceMillis == null || graceMillis in 0..60000)
        require(retireMillis == null || retireMillis in 1..60000)
    }
    private val death = IBinder.DeathRecipient {
        dead.complete(Unit)
        fail(IOException("Connection worker process exited"))
    }
    private val callback = object : Binder() {
        override fun onTransact(code: Int, data: Parcel, reply: Parcel?, flags: Int): Boolean {
            try {
                ConnectionWorkerIpc.validate(data, ConnectionWorkerIpc.EVENT, flags)
                require(code == 1)
                val revision = ConnectionWorkerIpc.generation(data)
                if (revision != generation) return true
                val bytes = ConnectionWorkerIpc.bytes(data)
                ConnectionWorkerIpc.end(data)
                if (ConnectionWorkerCodec.event(bytes) is ConnectionWorkerEvent.Ended) ended.complete(Unit)
                if (stopped.get()) return true
                check(events.trySend(bytes).isSuccess) { "Connection worker event backlog exceeded" }
                return true
            } catch (error: Exception) { fail(error); return false }
        }
    }
    private val connection = object : ServiceConnection {
        override fun onServiceConnected(name: ComponentName?, service: IBinder?) {
            if (service == null) { onNullBinding(name); return }
            connected(service)
        }
        override fun onServiceDisconnected(name: ComponentName?) = disconnected()
        override fun onBindingDied(name: ComponentName?) = disconnected()
        override fun onNullBinding(name: ComponentName?) = fail(IOException("Connection worker binding rejected"))
    }
    private fun connected(service: IBinder) {
        synchronized(guard) {
            if (!bound || !acceptingBinding || remote != null) return
            remote = service
        }
        try { service.linkToDeath(death, 0); ready.complete(service) }
        catch (error: Exception) { fail(error); ready.complete(service) }
    }
    private fun disconnected() {
        if (remote?.isBinderAlive == false) dead.complete(Unit)
        fail(IOException("Connection worker binding disconnected"))
    }
    private fun fail(error: Exception) {
        failure.compareAndSet(null, error)
        started.completeExceptionally(error)
        events.close(error)
        cancel()
    }
    override fun command(value: ConnectionWorkerCommand): Boolean {
        val encoded = ConnectionWorkerCodec.command(value)
        return !stopped.get() && commands.trySend(encoded).isSuccess
    }
    override suspend fun execute() {
        check(admitted.compareAndSet(false, true)) { "Connection worker already executed" }
        try {
            synchronized(guard) {
                check(!stopped.get()) { "Connection worker stopped" }
                outgoing = scope.launch(transmitDispatcher, start = CoroutineStart.LAZY) { transmit() }
                outgoing!!.start()
            }
            withTimeout(startMillis) { started.await() }
            observe()
        } finally { cancel() }
    }
    private suspend fun transmit() {
        try {
            bind()
            val service = withTimeout(bindMillis) { ready.await() }
            currentCoroutineContext().ensureActive()
            check(!stopped.get())
            startExposed = true // START may take effect before transact returns or throws.
            ConnectionWorkerIpc.command(service, ConnectionWorkerIpc.START, generation,
                ConnectionWorkerCodec.spec(spec), callback)
            started.complete(Unit)
            for (bytes in commands) {
                currentCoroutineContext().ensureActive()
                check(!stopped.get())
                ConnectionWorkerIpc.command(service, ConnectionWorkerIpc.CONTROL, generation, bytes)
            }
        } catch (error: Exception) {
            if (error !is CancellationException) fail(error)
            else started.completeExceptionally(error)
        }
    }
    private fun bind() {
        check(!stopped.get())
        bound = true
        try {
            val intent = Intent(app, ConnectionWorkerService::class.java).putExtra("connection_generation", generation)
            check(app.bindService(intent, connection, Context.BIND_AUTO_CREATE or Context.BIND_IMPORTANT)) { "Cannot bind connection worker" }
        } catch (error: Exception) { bound = false; throw error }
    }
    private suspend fun observe() {
        while (true) {
            failure.get()?.let { throw it }
            val event = ConnectionWorkerCodec.event(events.receive())
            if (event is ConnectionWorkerEvent.Ended) {
                event.failure?.let { throw IOException(it) }
                return
            }
            if (!stopped.get()) report(event)
        }
    }
    override fun cancel() {
        if (!stopped.compareAndSet(false, true)) return
        commands.close(); events.close(CancellationException("Connection worker stopped"))
        synchronized(guard) {
            outgoing?.cancel()
            stopping = scope.launch { stopAndExit() }
        }
    }
    private suspend fun stopAndExit() {
        val service = try { ready.await() } catch (_: Exception) { return }
        stopCall = scope.launch(Dispatchers.IO) { runCatching { ConnectionWorkerIpc.command(service, ConnectionWorkerIpc.STOP, generation) } }
        withTimeoutOrNull(grace) {
            select<Unit> { dead.onAwait {}; ended.onAwait {} }
        }
        if (!dead.isCompleted) {
            exitCall = scope.launch(Dispatchers.IO) { runCatching { ConnectionWorkerIpc.command(service, ConnectionWorkerIpc.EXIT, generation) } }
        }
    }
    override suspend fun retire() {
        cancel()
        if (retired) return
        withTimeout(retirementWait) {
            outgoing?.join()
            if (requiresDeath()) awaitDeath()
            else stopping?.cancel()
            stopping?.join(); stopCall?.join(); exitCall?.join()
            clean().await()
        }
        retired = true
        scope.cancel()
    }
    private fun requiresDeath(): Boolean = synchronized(guard) {
        if (startExposed || remote != null) true
        else { acceptingBinding = false; false }
    }
    private suspend fun awaitDeath() {
        while (!dead.isCompleted) {
            if (remote?.isBinderAlive == false) { dead.complete(Unit); return }
            delay(5)
        }
    }
    private fun clean(): Deferred<Unit> = synchronized(guard) {
        cleanup ?: scope.async<Unit>(Dispatchers.IO) {
            if (bound) { app.unbindService(connection); bound = false }
            remote?.unlinkToDeath(death, 0)
        }.also { cleanup = it }
    }
}
