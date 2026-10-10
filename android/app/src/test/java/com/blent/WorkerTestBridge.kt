package com.blent

import android.content.ComponentName
import android.os.*
import android.view.Surface
import kotlinx.coroutines.*
import org.robolectric.RuntimeEnvironment
import org.robolectric.Shadows
import java.util.concurrent.CopyOnWriteArrayList

/** Robolectric does not instantiate manifest bound services. This bridge runs
 * the same native factories and wire contracts while emulating Binder death.
 * Actual process separation is covered by the disposable-device suite. */
internal class WorkerTestBridge : Binder() {
    private val deaths = CopyOnWriteArrayList<IBinder.DeathRecipient>()
    private var scope = CoroutineScope(SupervisorJob() + Dispatchers.IO)
    private var owned: LifecycleRun? = null
    private var job: Job? = null
    private var revision = 0L
    override fun linkToDeath(recipient: IBinder.DeathRecipient, flags: Int) { deaths.add(recipient) }
    override fun unlinkToDeath(recipient: IBinder.DeathRecipient, flags: Int) = deaths.remove(recipient)
    override fun onTransact(code: Int, data: Parcel, reply: Parcel?, flags: Int): Boolean {
        WorkerIpc.validate(data, WorkerIpc.REQUEST)
        val generation = data.readLong()
        when (code) {
            WorkerIpc.START -> {
                revision = generation
                val task = MediaTaskJson.decode(requireNotNull(data.readString()))
                val remote = requireNotNull(data.readStrongBinder())
                val surface = if (data.readInt() == 1) Surface.CREATOR.createFromParcel(data) else null
                scope = CoroutineScope(SupervisorJob() + Dispatchers.IO)
                job = scope.launch {
                    var failure: String? = null
                    try {
                        owned = LocalMediaRuns.create(RuntimeEnvironment.getApplication(), task, surface) {
                            WorkerIpc.event(remote, generation, WorkerEventJson.encode(it))
                        }
                        owned!!.execute()
                    } catch (error: Exception) { if (error !is CancellationException) failure = error.message }
                    finally {
                        withContext(NonCancellable) {
                            owned?.cancel(); owned?.retire(); surface?.release()
                            WorkerIpc.event(remote, generation, WorkerEventJson.encode(WorkerEvent.Ended(failure)))
                        }
                    }
                }
            }
            WorkerIpc.STOP -> if (generation == revision) { owned?.cancel(); job?.cancel() }
            WorkerIpc.EXIT -> if (generation == revision) { owned?.cancel(); scope.cancel(); deaths.toList().forEach { it.binderDied() } }
        }
        return true
    }
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
            }
            return binder
        }
    }
}
