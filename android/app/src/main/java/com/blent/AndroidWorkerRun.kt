package com.blent

import android.content.*
import android.os.*
import android.view.Surface
import kotlinx.coroutines.*
import java.io.IOException
import java.util.concurrent.atomic.AtomicBoolean
import java.util.concurrent.atomic.AtomicLong

/** Main-process boundary adapter. Native work and payload sockets live remotely.
 * Framework binding loss is not process death. Failed retirement retains the
 * original binding and native control calls until actual exit can be observed. */
internal class AndroidWorkerRun(
    context: Context, private val task: MediaTask, private val surface: Surface? = null,
    private val report: (WorkerEvent) -> Unit = {},
    private val graceMillis: Long = 500, private val exitMillis: Long = 3000,
) : LifecycleRun {
    companion object { private val revisions = AtomicLong() }
    private val app = context.applicationContext
    private val generation = revisions.incrementAndGet()
    private val ready = CompletableDeferred<IBinder>()
    private val dead = CompletableDeferred<Unit>()
    private val mailbox = WorkerMailbox()
    private val stopped = AtomicBoolean()
    private val admitted = AtomicBoolean()
    private val control = CoroutineScope(SupervisorJob() + Dispatchers.Default)
    @Volatile private var remote: IBinder? = null
    @Volatile private var bound = false
    @Volatile private var started = false
    private var outgoing: Deferred<Unit>? = null
    private var cleanup: Deferred<Unit>? = null
    private var stopping: Job? = null
    private var stopCall: Job? = null
    private var exitCall: Job? = null
    private var retired = false
    private val bindingGate = Any()
    private var acceptingBinding = true
    @Volatile private var loss: IOException? = null
    init { require(graceMillis in 0..60000 && exitMillis in 1..60000) }
    private val death = IBinder.DeathRecipient { dead.complete(Unit); disconnected() }
    private val callback = object : Binder() {
        override fun onTransact(code: Int, data: Parcel, reply: Parcel?, flags: Int): Boolean {
            try {
                WorkerIpc.validate(data, WorkerIpc.EVENT)
                val revision = data.readLong()
                if (revision != generation || stopped.get()) return true
                check(mailbox.offer(requireNotNull(data.readString()))) { "Worker event backlog exceeded" }
                return true
            } catch (error: Exception) { mailbox.close(error); interrupt(); return false }
        }
    }
    private val connection = object : ServiceConnection {
        override fun onServiceConnected(name: ComponentName?, service: IBinder?) {
            if (service == null) { onNullBinding(name); return }
            synchronized(bindingGate) {
                if (!bound || !acceptingBinding || remote != null) return
                remote = service
            }
            try { service.linkToDeath(death, 0); ready.complete(service) }
            catch (error: Exception) { ready.completeExceptionally(error); disconnected() }
        }
        override fun onServiceDisconnected(name: ComponentName?) = disconnected()
        override fun onBindingDied(name: ComponentName?) = disconnected()
        override fun onNullBinding(name: ComponentName?) {
            ready.completeExceptionally(IOException("Media worker binding rejected")); disconnected()
        }
    }
    private fun disconnected() {
        if (remote?.isBinderAlive == false) dead.complete(Unit)
        ready.completeExceptionally(IOException("Media worker exited before readiness"))
        val failure = IOException("Media worker exited; start this resource again")
        loss = failure
        mailbox.close(failure)
        cancel()
    }
    override suspend fun execute() {
        currentCoroutineContext().ensureActive()
        check(admitted.compareAndSet(false, true)) { "Media worker already executed" }
        try {
            val start = synchronized(bindingGate) {
                check(!stopped.get()) { "Media worker request retired" }
                control.async(Dispatchers.IO, start = CoroutineStart.LAZY) { startNative() }.also {
                    outgoing = it
                    it.start()
                }
            }
            start.await()
            observe()
        } finally { cancel() }
    }
    private suspend fun startNative() {
        bind()
        val service = withTimeout(5000) { ready.await() }
        currentCoroutineContext().ensureActive()
        check(!stopped.get()) { "Media worker request retired" }
        started = true // START may take effect before its native call returns.
        WorkerIpc.command(service, WorkerIpc.START, generation, MediaTaskJson.encode(task), callback, surface)
    }
    private fun bind() {
        check(!stopped.get()) { "Media worker request retired" }
        var flags = Context.BIND_AUTO_CREATE or Context.BIND_IMPORTANT
        if (Build.VERSION.SDK_INT >= 30) flags = flags or Context.BIND_INCLUDE_CAPABILITIES
        bound = true
        try { check(app.bindService(Intent(app, serviceClass(task)).putExtra("worker_generation", generation), connection, flags)) { "Cannot bind media worker" } }
        catch (error: Exception) { bound = false; throw error }
    }
    private suspend fun observe() {
        while (!stopped.get()) {
            val event = WorkerEventJson.decode(mailbox.receive())
            if (event is WorkerEvent.Ended) {
                event.failure?.let { throw IOException(it) }
                return
            }
            if (!stopped.get()) report(event)
        }
        throw loss ?: CancellationException("Media worker stopped")
    }
    override fun cancel() {
        stopped.set(true)
        synchronized(bindingGate) { outgoing?.cancel() }
        mailbox.close(CancellationException("Media worker stopped"))
        interrupt()
    }
    @Synchronized private fun interrupt() {
        if (stopping != null) return
        stopping = control.launch { stopAndExit() }
    }
    private suspend fun stopAndExit() {
        val service = try { ready.await() } catch (_: Exception) { remote ?: return }
        // Separate jobs: a stuck STOP transaction cannot consume the exit timer.
        stopCall = control.launch(Dispatchers.IO) { runCatching { WorkerIpc.command(service, WorkerIpc.STOP, generation) } }
        if (withTimeoutOrNull(graceMillis) { dead.await(); true } != true) {
            exitCall = control.launch(Dispatchers.IO) { runCatching { WorkerIpc.command(service, WorkerIpc.EXIT, generation) } }
        }
    }
    override suspend fun retire() {
        cancel()
        if (retired) return
        withTimeout(exitMillis) {
            outgoing?.join()
            if (requiresDeath()) awaitDeath() else stopping?.cancel()
            stopping?.join(); stopCall?.join(); exitCall?.join()
            clean().await()
        }
        retired = true
        control.cancel()
    }
    private fun requiresDeath(): Boolean = synchronized(bindingGate) {
        if (started || remote != null) true
        else { acceptingBinding = false; false }
    }
    private suspend fun awaitDeath() {
        while (!dead.isCompleted) {
            if (remote?.isBinderAlive == false) { dead.complete(Unit); return }
            delay(5)
        }
    }
    private fun clean(): Deferred<Unit> = synchronized(bindingGate) {
        cleanup ?: control.async<Unit>(Dispatchers.IO) {
            if (bound) { app.unbindService(connection); bound = false }
            remote?.unlinkToDeath(death, 0)
        }.also { cleanup = it }
    }
    internal fun serviceClass(value: MediaTask): Class<out MediaWorkerService> = when (value) {
        is MediaTask.Display -> DisplayWorkerService::class.java
        is MediaTask.Camera -> CameraWorkerService::class.java
        is MediaTask.Audio -> if (value.endpoint.direction == 1) MicrophoneWorkerService::class.java else SpeakerWorkerService::class.java
        is MediaTask.Metadata -> MetadataWorkerService::class.java
    }
}
