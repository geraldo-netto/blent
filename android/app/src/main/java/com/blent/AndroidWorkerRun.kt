package com.blent

import android.content.*
import android.os.*
import android.view.Surface
import kotlinx.coroutines.*
import java.io.IOException
import java.util.concurrent.atomic.AtomicBoolean
import java.util.concurrent.atomic.AtomicLong

/** Main-process boundary adapter. Native work and payload sockets live remotely. */
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
    private val control = CoroutineScope(SupervisorJob() + Dispatchers.Default)
    @Volatile private var remote: IBinder? = null
    @Volatile private var bound = false
    @Volatile private var started = false
    private var stopping: Job? = null
    private val death = IBinder.DeathRecipient { died() }
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
            if (!bound || remote != null) return
            if (service == null) { onNullBinding(name); return }
            remote = service
            try { service.linkToDeath(death, 0); ready.complete(service) }
            catch (error: Exception) { ready.completeExceptionally(error); died() }
        }
        override fun onServiceDisconnected(name: ComponentName?) = died()
        override fun onBindingDied(name: ComponentName?) = died()
        override fun onNullBinding(name: ComponentName?) {
            ready.completeExceptionally(IOException("Media worker binding rejected")); died()
        }
    }
    private fun died() {
        ready.completeExceptionally(IOException("Media worker exited before readiness"))
        dead.complete(Unit)
        mailbox.close(IOException("Media worker exited; start this resource again"))
    }
    override suspend fun execute() {
        currentCoroutineContext().ensureActive()
        check(!stopped.get()) { "Media worker request retired" }
        bind()
        val service = withTimeout(5000) { ready.await() }
        currentCoroutineContext().ensureActive()
        withContext(Dispatchers.IO) {
            check(!stopped.get()) { "Media worker request retired" }
            started = true
            WorkerIpc.command(service, WorkerIpc.START, generation, MediaTaskJson.encode(task), callback, surface)
        }
        observe()
    }
    private fun bind() {
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
    }
    override fun cancel() {
        stopped.set(true)
        mailbox.close(CancellationException("Media worker stopped"))
        interrupt()
    }
    @Synchronized private fun interrupt() {
        if (stopping != null) return
        stopping = control.launch { stopAndExit() }
    }
    private suspend fun stopAndExit() {
        val service = remote ?: return
        // Separate jobs: a stuck STOP transaction cannot consume the exit timer.
        val stop = control.launch(Dispatchers.IO) { runCatching { WorkerIpc.command(service, WorkerIpc.STOP, generation) } }
        if (withTimeoutOrNull(graceMillis) { dead.await(); true } != true) {
            withContext(Dispatchers.IO) { runCatching { WorkerIpc.command(service, WorkerIpc.EXIT, generation) } }
        }
        stop.cancel()
    }
    override suspend fun retire() {
        cancel()
        try {
            if (started) withTimeout(exitMillis) { dead.await() }
        } finally {
            // Never admit a replacement on a mere timeout: ResourceLane keeps
            // failed retirement quarantined. Unbinding also requests worker exit.
            try { unbind() } finally { control.cancel() }
        }
    }
    private fun unbind() {
        if (bound) { bound = false; app.unbindService(connection) }
        remote?.unlinkToDeath(death, 0)
    }
    internal fun serviceClass(value: MediaTask): Class<out MediaWorkerService> = when (value) {
        is MediaTask.Display -> DisplayWorkerService::class.java
        is MediaTask.Camera -> CameraWorkerService::class.java
        is MediaTask.Audio -> if (value.endpoint.direction == 1) MicrophoneWorkerService::class.java else SpeakerWorkerService::class.java
        is MediaTask.Metadata -> MetadataWorkerService::class.java
    }
}
