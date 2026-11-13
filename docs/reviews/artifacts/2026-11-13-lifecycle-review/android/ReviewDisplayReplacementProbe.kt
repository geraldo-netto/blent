package com.blent

import android.content.*
import android.graphics.SurfaceTexture
import android.os.*
import android.view.*
import kotlinx.coroutines.*
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.Robolectric
import org.robolectric.RobolectricTestRunner
import org.robolectric.RuntimeEnvironment
import org.robolectric.annotation.Config
import java.lang.reflect.Proxy
import java.util.concurrent.CopyOnWriteArrayList
import java.util.concurrent.atomic.AtomicInteger

/** Caches a bound-service Binder as required by the Android service contract. */
private class CachedDisplayContext(private val service: WorkerProbeService) : ContextWrapper(RuntimeEnvironment.getApplication()) {
    private var endpoint: CachedDisplayEndpoint? = null
    val binds = AtomicInteger()
    override fun getApplicationContext(): Context = this
    @Synchronized override fun bindService(intent: Intent, connection: ServiceConnection, flags: Int): Boolean {
        val original = endpoint ?: CachedDisplayEndpoint(service.onBind(intent), service).also { endpoint = it }
        binds.incrementAndGet()
        connection.onServiceConnected(intent.component, original)
        return true
    }
    override fun unbindService(connection: ServiceConnection) {}
    fun die() { endpoint?.die() }
}

private class CachedDisplayEndpoint(private val target: IBinder, private val service: WorkerProbeService) : Binder() {
    private val deaths = CopyOnWriteArrayList<IBinder.DeathRecipient>()
    @Volatile private var alive = true
    override fun isBinderAlive(): Boolean = alive
    override fun linkToDeath(recipient: IBinder.DeathRecipient, flags: Int) { deaths.add(recipient) }
    override fun unlinkToDeath(recipient: IBinder.DeathRecipient, flags: Int): Boolean = deaths.remove(recipient)
    override fun onTransact(code: Int, data: Parcel, reply: Parcel?, flags: Int): Boolean {
        val accepted = target.transact(code, data, reply, flags)
        if (service.terminations.get() > 0) die()
        return accepted
    }
    fun die() { alive = false; deaths.forEach { it.binderDied() } }
}

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [27, 34])
class ReviewDisplayReplacementProbe {
    private fun view(surface: Surface) = object : SurfaceView(RuntimeEnvironment.getApplication()) {
        override fun getHolder(): SurfaceHolder = Proxy.newProxyInstance(SurfaceHolder::class.java.classLoader,
            arrayOf(SurfaceHolder::class.java)) { _, method, _ -> if (method.name == "getSurface") surface else null } as SurfaceHolder
    }
    private suspend fun until(condition: () -> Boolean) { while (!condition()) delay(1) }

    @Test fun replacementDisplayMustWaitForOriginalServiceRetirement() = checkDisplay(true)
    @Test fun sameDisplayWaitsForOriginalServiceRetirement() = checkDisplay(false)

    private fun checkDisplay(recreate: Boolean) = runBlocking {
        val controller = Robolectric.buildService(WorkerProbeService::class.java).create()
        val service = controller.get()
        val context = CachedDisplayContext(service)
        val scope = CoroutineScope(SupervisorJob() + Dispatchers.Unconfined)
        val entered = CompletableDeferred<Unit>()
        val release = CompletableDeferred<Unit>()
        val retiring = CompletableDeferred<Unit>()
        val surface = Surface(SurfaceTexture(0))
        service.factory = { _, _, _ -> object : LifecycleRun {
            override suspend fun execute() { entered.complete(Unit); awaitCancellation() }
            override fun cancel() {}
            override suspend fun retire() { retiring.complete(Unit); release.await() }
        } }
        val workers = CopyOnWriteArrayList<AndroidWorkerRun>()
        fun receiver() = RemoteDisplay(context, scope) { task, output, report ->
            AndroidWorkerRun(context, task, output, report, graceMillis = 60000).also { workers.add(it) }
        }.also { it.token = "a".repeat(64); it.setSurface(view(surface)) }
        val original = receiver()
        val replacement = if (recreate) receiver() else original
        try {
            original.start(); entered.await()
            original.stop(); retiring.await()
            replacement.start()
            if (recreate) {
                until { context.binds.get() == 2 }
                until { replacement.startupStatus.value?.contains("Starting") != true }
            }
            println("REPLACEMENT recreate=$recreate binds=${context.binds.get()} originalRetirementReleased=${release.isCompleted} workerTerminations=${service.terminations.get()} replacementStatus=${replacement.startupStatus.value}")
            assertEquals("Replacement Activity must not bind the retiring singleton display worker", 1, context.binds.get())
        } finally {
            original.stop(); replacement.stop(); release.complete(Unit); context.die()
            workers.forEach { it.retire() }
            scope.cancel(); controller.destroy(); surface.release()
        }
    }
}
