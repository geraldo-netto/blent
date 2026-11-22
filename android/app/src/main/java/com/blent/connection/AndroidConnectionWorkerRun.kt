package com.blent.connection

import android.content.Context
import android.content.Intent
import android.os.Binder
import android.os.IBinder
import android.os.Parcel
import kotlinx.coroutines.*
import kotlinx.coroutines.channels.Channel
import java.io.IOException
import java.util.concurrent.atomic.AtomicBoolean
import com.blent.WorkerProcessLifetime
import com.blent.WorkerGenerations
import java.util.concurrent.atomic.AtomicReference

/** Main-process adapter. Native IPC jobs retain ownership independently of their
 * awaiting caller. Ended/framework disconnect never substitutes for Binder death.
 * Failed retirement retains the binding and jobs; a later retry can observe exit. */
internal class AndroidConnectionWorkerRun(
    context: Context,
    private val launch: ConnectionProcessLaunch,
    private val report: (ConnectionWorkerEvent) -> Unit = {},
    private val bindMillis: Long = 5000,
    private val startMillis: Long = 5000,
    graceMillis: Long? = null,
    retireMillis: Long? = null,
    generations: WorkerGenerations = sharedGenerations,
    private val transmitDispatcher: CoroutineDispatcher = Dispatchers.IO,
    private val trustMillis: Long = 5000,
) : ConnectionWorker {
    constructor(
        context: Context, spec: ConnectionWorkerSpec,
        report: (ConnectionWorkerEvent) -> Unit = {},
        bindMillis: Long = 5000, startMillis: Long = 5000,
        graceMillis: Long? = null, retireMillis: Long? = null,
        generations: WorkerGenerations = sharedGenerations,
        transmitDispatcher: CoroutineDispatcher = Dispatchers.IO,
    ) : this(context, ConnectionProcessLaunch.Connect(spec), report, bindMillis, startMillis,
        graceMillis, retireMillis, generations, transmitDispatcher)

    companion object { private val sharedGenerations = WorkerGenerations() }
    // Native retirement plus the maximum supported actor timer cadence. Explicit
    // overrides remain bounded to 60 seconds. Ended permits EXIT, never retirement.
    private val grace = graceMillis ?: (launch.retirementTimeoutMs + 1000)
    private val retirementWait = retireMillis ?: (grace + 5000)
    private val app = context.applicationContext
    private val generation = generations.next()
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Default)
    private val admitted = AtomicBoolean()
    private val stopped = AtomicBoolean()
    private val failure = AtomicReference<Exception?>()
    private val started = CompletableDeferred<Unit>()
    private val commands = Channel<ByteArray>(32)
    private val cleanupCommands = Channel<ByteArray>(32)
    private val events = Channel<ByteArray>(32)
    private var outgoing: Job? = null
    private var cleanupRelay: Job? = null
    init {
        require(bindMillis in 1..60000 && startMillis in 1..60000)
        require(trustMillis in 1..60000)
        require(graceMillis == null || graceMillis in 0..60000)
        require(retireMillis == null || retireMillis in 1..60000)
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
                if (ConnectionWorkerCodec.event(bytes) is ConnectionWorkerEvent.Ended) process.ended.complete(Unit)
                if (stopped.get()) return true
                check(events.trySend(bytes).isSuccess) { "Connection worker event backlog exceeded" }
                return true
            } catch (error: Exception) { fail(error); return false }
        }
    }
    private val process = WorkerProcessLifetime(app, scope, grace,
        disconnected = ::disconnected,
        rejected = { fail(IOException("Connection worker binding rejected")) },
        linkFailed = ::linkFailed,
        stopCommand = { ConnectionWorkerIpc.command(it, ConnectionWorkerIpc.STOP, generation) },
        exitCommand = { ConnectionWorkerIpc.command(it, ConnectionWorkerIpc.EXIT, generation) })
    private fun linkFailed(error: Exception, service: IBinder) {
        fail(error)
        process.ready.complete(service)
    }
    private fun disconnected() {
        process.observeDeath()
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
        if (value.isConsumerRetirement()) return cleanupCommand(encoded)
        return !stopped.get() && commands.trySend(encoded).isSuccess
    }
    /** Only retained proof can use this independent bounded native relay. It
     * never binds/starts a replacement and retains blocked IPC through retire. */
    private fun cleanupCommand(bytes: ByteArray): Boolean = synchronized(process.gate) {
        if (process.retired || process.dead.isCompleted || process.ended.isCompleted || !process.startExposed) return@synchronized false
        val service = process.remote ?: return@synchronized false
        val accepted = cleanupCommands.trySend(bytes).isSuccess
        if (accepted && cleanupRelay == null) {
            cleanupRelay = scope.launch(transmitDispatcher, start = CoroutineStart.LAZY) { cleanupControls(service) }
            checkNotNull(cleanupRelay).start()
        }
        accepted
    }
    private suspend fun cleanupControls(service: IBinder) {
        try {
            for (bytes in cleanupCommands) {
                if (stopped.get()) { process.awaitStopCall() }
                if (!process.dead.isCompleted && !process.ended.isCompleted) {
                    ConnectionWorkerIpc.command(service, ConnectionWorkerIpc.CONTROL, generation, bytes)
                }
            }
        } catch (error: Exception) { if (error !is CancellationException) fail(error) }
    }
    override suspend fun execute() {
        check(admitted.compareAndSet(false, true)) { "Connection worker already executed" }
        try {
            synchronized(process.gate) {
                check(!stopped.get()) { "Connection worker stopped" }
                outgoing = scope.launch(transmitDispatcher, start = CoroutineStart.LAZY) { transmit() }
                outgoing!!.start()
            }
            connectionWorkerBudget("startup", startMillis) { started.await() }
            observeOperation()
        } finally { cancel() }
    }
    private suspend fun transmit() {
        try {
            bind()
            val service = connectionWorkerBudget("binding", bindMillis) { process.ready.await() }
            currentCoroutineContext().ensureActive()
            check(!stopped.get())
            process.startExposed = true // START may take effect before transact returns or throws.
            start(service)
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
    private fun start(service: IBinder) {
        when (val selected = launch) {
            is ConnectionProcessLaunch.Connect -> ConnectionWorkerIpc.command(service, ConnectionWorkerIpc.START,
                generation, ConnectionWorkerCodec.spec(selected.spec), callback)
            is ConnectionProcessLaunch.Trust -> ConnectionWorkerIpc.command(service, ConnectionWorkerIpc.START_TRUST,
                generation, SavedTrustCodec.request(selected.request), callback)
        }
    }
    private fun bind() {
        check(!stopped.get())
        val intent = Intent(app, ConnectionWorkerService::class.java).putExtra("connection_generation", generation)
        process.bind(intent, Context.BIND_AUTO_CREATE or Context.BIND_IMPORTANT)
    }
    private suspend fun observeOperation() {
        if (launch is ConnectionProcessLaunch.Trust) connectionWorkerBudget("saved-trust operation", trustMillis) { observe() }
        else observe()
    }
    private suspend fun observe() {
        while (true) {
            failure.get()?.let { throw it }
            val event = ConnectionWorkerCodec.event(events.receive())
            if (event is ConnectionWorkerEvent.Ended) {
                event.failure?.let { throw IOException(it) }
                return
            }
            check(expected(event)) { "Unexpected event for private worker operation" }
            if (!stopped.get()) report(event)
        }
    }
    private fun expected(event: ConnectionWorkerEvent): Boolean = when (launch) {
        is ConnectionProcessLaunch.Connect -> event !is ConnectionWorkerEvent.SavedComputers
        is ConnectionProcessLaunch.Trust -> event is ConnectionWorkerEvent.SavedComputers
    }
    override fun cancel() {
        if (!stopped.compareAndSet(false, true)) return
        commands.close(); events.close(CancellationException("Connection worker stopped"))
        synchronized(process.gate) {
            outgoing?.cancel()
            process.stop()
        }
    }
    override suspend fun retire() {
        cancel()
        process.retire(retirementWait, original = { outgoing?.join() }, drain = {
            cleanupCommands.close()
            cleanupRelay?.join()
        })
    }
}
