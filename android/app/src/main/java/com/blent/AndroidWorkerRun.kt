package com.blent

import android.content.*
import android.os.*
import android.view.Surface
import kotlinx.coroutines.*
import java.io.IOException
import java.util.concurrent.atomic.AtomicBoolean

/** Main-process boundary adapter. Native work and payload sockets live remotely.
 * Framework binding loss is not process death. Failed retirement retains the
 * original binding and native control calls until actual exit can be observed. */
internal class AndroidWorkerRun(
    context: Context, private val task: MediaTask, private val surface: Surface? = null,
    private val report: (WorkerEvent) -> Unit = {},
    private val graceMillis: Long = 500, private val exitMillis: Long = 3000,
    generations: WorkerGenerations = sharedGenerations,
) : LifecycleRun {
    companion object { private val sharedGenerations = WorkerGenerations() }
    private val generation = generations.next()
    private val app = context.applicationContext
    private val mailbox = WorkerMailbox()
    private val stopped = AtomicBoolean()
    private val admitted = AtomicBoolean()
    private val control = CoroutineScope(SupervisorJob() + Dispatchers.Default)
    private var outgoing: Deferred<Unit>? = null
    @Volatile private var loss: IOException? = null
    init { require(graceMillis in 0..60000 && exitMillis in 1..60000) }
    private val callback = object : Binder() {
        override fun onTransact(code: Int, data: Parcel, reply: Parcel?, flags: Int): Boolean {
            try {
                WorkerIpc.validate(data, WorkerIpc.EVENT)
                val revision = data.readLong()
                if (revision != generation) return true
                receiveEvent(requireNotNull(data.readString()))
                return true
            } catch (error: Exception) { mailbox.close(error); interrupt(); return false }
        }
    }
    private fun receiveEvent(text: String) {
        val event = WorkerEventJson.decode(text)
        if (event is WorkerEvent.Ended) process.ended.complete(Unit)
        if (!stopped.get()) check(mailbox.offer(WorkerMessage(event, text))) { "Worker event backlog exceeded" }
    }
    private val process = WorkerProcessLifetime(app, control, graceMillis,
        disconnected = ::disconnected,
        rejected = {
            processReadyFailure(IOException("Media worker binding rejected"))
            disconnected()
        },
        linkFailed = { error, _ -> processReadyFailure(error); disconnected() },
        stopCommand = { WorkerIpc.command(it, WorkerIpc.STOP, generation) },
        exitCommand = { WorkerIpc.command(it, WorkerIpc.EXIT, generation) })
    private fun processReadyFailure(error: Exception) { process.ready.completeExceptionally(error) }
    private fun disconnected() {
        process.observeDeath()
        process.ready.completeExceptionally(IOException("Media worker exited before readiness"))
        val failure = IOException("Media worker exited; start this resource again")
        loss = failure
        mailbox.close(failure)
        cancel()
    }
    override suspend fun execute() {
        currentCoroutineContext().ensureActive()
        check(admitted.compareAndSet(false, true)) { "Media worker already executed" }
        try {
            val start = synchronized(process.gate) {
                check(!stopped.get()) { "Media worker request retired" }
                control.async(Dispatchers.IO, start = CoroutineStart.LAZY) { startNative() }.also {
                    outgoing = it
                    it.start()
                }
            }
            start.await()
            withContext(Dispatchers.Default) { observe() }
        } finally { cancel() }
    }
    private suspend fun startNative() {
        bind()
        val service = withTimeout(5000) { process.ready.await() }
        currentCoroutineContext().ensureActive()
        check(!stopped.get()) { "Media worker request retired" }
        process.startExposed = true // START may take effect before its native call returns.
        WorkerIpc.command(service, WorkerIpc.START, generation, MediaTaskJson.encode(task), callback, surface)
    }
    private fun bind() {
        check(!stopped.get()) { "Media worker request retired" }
        var flags = Context.BIND_AUTO_CREATE or Context.BIND_IMPORTANT
        if (Build.VERSION.SDK_INT >= 30) flags = flags or Context.BIND_INCLUDE_CAPABILITIES
        process.bind(Intent(app, serviceClass(task)).putExtra("worker_generation", generation), flags)
    }
    private suspend fun observe() {
        while (!stopped.get()) {
            val event = mailbox.receive().event
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
        synchronized(process.gate) { outgoing?.cancel() }
        mailbox.close(CancellationException("Media worker stopped"))
        interrupt()
    }
    private fun interrupt() { process.stop() }
    override suspend fun retire() {
        cancel()
        process.retire(exitMillis, original = { outgoing?.join() })
    }
    internal fun serviceClass(value: MediaTask): Class<out MediaWorkerService> = when (value) {
        is MediaTask.Display -> DisplayWorkerService::class.java
        is MediaTask.Camera -> CameraWorkerService::class.java
        is MediaTask.Audio -> if (value.endpoint.direction == 1) MicrophoneWorkerService::class.java else SpeakerWorkerService::class.java
        is MediaTask.Metadata -> MetadataWorkerService::class.java
        is MediaTask.Wake -> PowerWorkerService::class.java
    }
}
