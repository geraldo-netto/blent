package com.blent

import android.content.ComponentName
import android.content.Intent
import android.os.*
import android.view.Surface
import kotlinx.coroutines.*
import org.robolectric.RuntimeEnvironment
import org.robolectric.Shadows
import java.util.concurrent.CopyOnWriteArrayList
import java.util.concurrent.atomic.AtomicBoolean

/** Models local native factories plus original execution/cleanup settlement.
 * It cannot emulate forced OS death of an uncooperative native thread; unknown
 * cleanup therefore never publishes a fictitious process fence. */
internal class WorkerTestBridge(
    private val create: (MediaTask, Surface?, (WorkerEvent) -> Unit) -> LifecycleRun = { task, surface, report ->
        LocalMediaRuns.create(RuntimeEnvironment.getApplication(), task, surface, report)
    },
) : Binder() {
    private class Execution(val revision: Long, val task: MediaTask, val remote: IBinder, val surface: Surface?) {
        val scope = CoroutineScope(SupervisorJob() + Dispatchers.IO)
        val control = CoroutineScope(SupervisorJob() + Dispatchers.IO)
        val stopped = AtomicBoolean()
        val settled = CompletableDeferred<Throwable?>()
        @Volatile var owned: LifecycleRun? = null
        lateinit var job: Job
        var exit: Job? = null
        var stopCall: Job? = null
        var closing = false
        @Volatile var exited = false
    }
    private val deaths = CopyOnWriteArrayList<IBinder.DeathRecipient>()
    private var current: Execution? = null
    private var lastRevision = 0L
    override fun linkToDeath(recipient: IBinder.DeathRecipient, flags: Int) { deaths.add(recipient) }
    override fun unlinkToDeath(recipient: IBinder.DeathRecipient, flags: Int) = deaths.remove(recipient)
    override fun onTransact(code: Int, data: Parcel, reply: Parcel?, flags: Int): Boolean = try {
        WorkerIpc.validate(data, WorkerIpc.REQUEST)
        val revision = data.readLong()
        when (code) {
            WorkerIpc.START -> start(revision, data)
            WorkerIpc.STOP -> original(revision)?.let { stop(it); true } ?: false
            WorkerIpc.EXIT -> original(revision)?.let { exit(it); true } ?: false
            else -> false
        }
    } catch (_: Exception) { false }
    @Synchronized private fun original(revision: Long): Execution? = current?.takeIf { it.revision == revision }
    @OptIn(ExperimentalCoroutinesApi::class)
    @Synchronized private fun start(revision: Long, data: Parcel): Boolean {
        check(revision > lastRevision && current?.exited != false)
        val task = MediaTaskJson.decode(requireNotNull(data.readString()))
        val remote = requireNotNull(data.readStrongBinder())
        val surface = surface(data)
        val original = Execution(revision, task, remote, surface)
        current = original; lastRevision = revision
        original.job = original.scope.launch(start = CoroutineStart.ATOMIC) { execute(original) }
        return true
    }
    private fun surface(data: Parcel): Surface? = when (data.readInt()) {
        0 -> null
        1 -> Surface.CREATOR.createFromParcel(data)
        else -> error("Invalid bridge Surface marker")
    }
    private suspend fun execute(original: Execution) {
        var failure: Exception? = null
        try {
            if (original.stopped.get()) return
            val owned = create(original.task, original.surface) { WorkerIpc.event(original.remote, original.revision, WorkerEventJson.encode(it)) }
            original.owned = owned
            if (!original.stopped.get()) owned.execute()
        } catch (error: Exception) { if (error !is CancellationException) failure = error }
        finally { settle(original, failure) }
    }
    private suspend fun settle(original: Execution, failure: Exception?) = withContext(NonCancellable) {
        val cleanup = close(original)
        original.settled.complete(cleanup)
        WorkerIpc.event(original.remote, original.revision, WorkerEventJson.encode(WorkerEvent.Ended((cleanup ?: failure)?.message)))
        if (cleanup != null && cleanup !is Exception) throw cleanup
    }
    private suspend fun close(original: Execution): Throwable? {
        val stop = synchronized(original) { original.closing = true; original.stopCall }
        stop?.join()
        val failures = ArrayList<Throwable>()
        attempt(failures) { original.owned?.cancel() }
        attempt(failures) { original.owned?.retire() }
        attempt(failures) { original.surface?.release() }
        val first = failures.firstOrNull() ?: return null
        failures.drop(1).forEach { if (it !== first) first.addSuppressed(it) }
        return first
    }
    private suspend fun attempt(failures: MutableList<Throwable>, action: suspend () -> Unit) {
        try { action() } catch (failure: Throwable) { failures.add(failure) }
    }
    private fun stop(original: Execution) = synchronized(original) {
        if (!original.stopped.compareAndSet(false, true)) return@synchronized
        if (!original.closing) {
            original.stopCall = original.control.launch(start = CoroutineStart.LAZY) {
                try { original.owned?.cancel() } catch (_: Exception) { /* Actual retire still must prove settlement. */ }
            }
        }
        original.scope.cancel()
        original.stopCall?.start()
    }
    @Synchronized private fun exit(original: Execution) {
        if (original.exit != null) return
        stop(original)
        original.exit = original.control.launch(start = CoroutineStart.LAZY) {
            original.job.join()
            if (original.settled.await() != null) return@launch
            val recipients = deaths.toList()
            original.exited = true
            try { recipients.forEach { it.binderDied() } }
            finally { original.control.cancel() }
        }
        original.exit!!.start()
    }
    suspend fun awaitExitAttempt(revision: Long) { original(revision)?.exit?.join() }
    companion object {
        fun install(): WorkerTestBridge {
            // Robolectric reuses class loaders across fresh Application objects.
            // A real process has one Application and one query owner.
            val previous = org.robolectric.util.ReflectionHelpers.getStaticField<WorkerQueries?>(RemoteMetadata::class.java, "owner")
            kotlinx.coroutines.runBlocking {
                withTimeout(4000) { while (previous?.busy == true) delay(1) }
            }
            org.robolectric.util.ReflectionHelpers.setStaticField(RemoteMetadata::class.java, "owner", null)
            val app = RuntimeEnvironment.getApplication()
            val binder = WorkerTestBridge()
            Shadows.shadowOf(app).apply {
                setBindServiceCallsOnServiceConnectedDirectly(true)
                setUnbindServiceCallsOnServiceDisconnected(false)
                setComponentNameAndServiceForBindService(ComponentName(app, MetadataWorkerService::class.java), binder)
                val types = listOf(DisplayWorkerService::class.java, CameraWorkerService::class.java,
                    MicrophoneWorkerService::class.java, SpeakerWorkerService::class.java, MetadataWorkerService::class.java, PowerWorkerService::class.java)
                types.forEach { type ->
                    val native = if (type == MetadataWorkerService::class.java) binder else WorkerTestBridge()
                    setComponentNameAndServiceForBindServiceForIntent(Intent(app, type), ComponentName(app, type), native)
                }
            }
            return binder
        }
    }
}
