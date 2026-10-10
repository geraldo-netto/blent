package com.blent

import android.app.Service
import android.content.Intent
import android.os.*
import android.view.Surface
import kotlinx.coroutines.*
import java.util.concurrent.atomic.AtomicBoolean

/** One private process, one admitted native owner. Binder control never calls
 * native media operations. Process exit, not a timeout, proves forced retirement. */
abstract class MediaWorkerService : Service() {
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.IO)
    private val admitted = AtomicBoolean()
    private val stopped = AtomicBoolean()
    private var generation = 0L
    @Volatile private var run: LifecycleRun? = null
    private var execution: Job? = null
    private var callback: IBinder? = null
    private val clientDeath = IBinder.DeathRecipient { terminate() }
    private val events = WorkerMailbox()
    internal abstract fun accepts(task: MediaTask): Boolean
    internal open fun createRun(task: MediaTask, surface: Surface?, report: (WorkerEvent) -> Unit): LifecycleRun =
        LocalMediaRuns.create(this, task, surface, report)
    internal open fun terminate() { Process.killProcess(Process.myPid()) }

    private val binder = object : Binder() {
        override fun onTransact(code: Int, data: Parcel, reply: Parcel?, flags: Int): Boolean {
            try {
                WorkerIpc.validate(data, WorkerIpc.REQUEST)
                val revision = data.readLong()
                return command(code, revision, data)
            } catch (_: Exception) { terminate(); return false }
        }
    }
    @Synchronized private fun command(code: Int, revision: Long, data: Parcel): Boolean {
        if (code == WorkerIpc.START) { start(revision, data); return true }
        if (revision != generation) return false
        when (code) {
            WorkerIpc.STOP -> stop()
            WorkerIpc.EXIT -> terminate()
            else -> return false
        }
        return true
    }
    private fun start(revision: Long, data: Parcel) {
        check(revision == generation && revision > 0 && !stopped.get() && admitted.compareAndSet(false, true))
        val task = MediaTaskJson.decode(requireNotNull(data.readString()))
        require(accepts(task)) { "Wrong resource process" }
        val remote = requireNotNull(data.readStrongBinder())
        val surface = when (data.readInt()) { 0 -> null; 1 -> Surface.CREATOR.createFromParcel(data); else -> error("Invalid Surface marker") }
        generation = revision
        callback = remote
        remote.linkToDeath(clientDeath, 0)
        scope.launch { deliver(remote, revision) }
        execution = scope.launch { execute(task, surface) }
    }
    private suspend fun deliver(remote: IBinder, revision: Long) {
        try { while (currentCoroutineContext().isActive) WorkerIpc.event(remote, revision, events.receive()) }
        catch (_: Exception) { terminate() }
    }
    private fun publish(event: WorkerEvent) {
        try { check(events.offer(WorkerEventJson.encode(event))) }
        catch (_: Exception) { terminate() }
    }
    private suspend fun execute(task: MediaTask, surface: Surface?) {
        var failure: Exception? = null
        try {
            val owned = createRun(task, surface, ::publish)
            run = owned
            if (stopped.get()) return
            owned.execute()
        } catch (error: Exception) { if (error !is CancellationException) failure = error }
        finally {
            withContext(NonCancellable) {
                try { run?.cancel() } catch (error: Exception) { failure = error }
                try { run?.retire() } catch (error: Exception) { failure = error }
                try { surface?.release() } catch (error: Exception) { failure = error }
                publish(WorkerEvent.Ended(failure?.message))
            }
        }
    }
    private fun stop() {
        if (!stopped.compareAndSet(false, true)) return
        execution?.cancel()
        scope.launch { try { run?.cancel() } catch (_: Exception) { terminate() } }
    }
    override fun onBind(intent: Intent): IBinder {
        generation = intent.getLongExtra("worker_generation", 0)
        require(generation > 0)
        return binder
    }
    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int) = START_NOT_STICKY
    override fun onUnbind(intent: Intent): Boolean { terminate(); return false }
    override fun onDestroy() {
        try { callback?.unlinkToDeath(clientDeath, 0) }
        finally {
            scope.cancel(); events.close()
            try { terminate() } finally { super.onDestroy() }
        }
    }
}

class DisplayWorkerService : MediaWorkerService() { override fun accepts(task: MediaTask) = task is MediaTask.Display }
class CameraWorkerService : MediaWorkerService() { override fun accepts(task: MediaTask) = task is MediaTask.Camera }
class MicrophoneWorkerService : MediaWorkerService() { override fun accepts(task: MediaTask) = task is MediaTask.Audio && task.endpoint.direction == 1 }
class SpeakerWorkerService : MediaWorkerService() { override fun accepts(task: MediaTask) = task is MediaTask.Audio && task.endpoint.direction == 2 }
class MetadataWorkerService : MediaWorkerService() { override fun accepts(task: MediaTask) = task is MediaTask.Metadata }
