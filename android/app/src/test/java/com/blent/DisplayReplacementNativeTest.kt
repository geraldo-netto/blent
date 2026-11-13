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
import org.robolectric.android.controller.ServiceController
import org.robolectric.annotation.Config
import org.robolectric.util.ReflectionHelpers
import java.lang.reflect.Proxy
import java.util.concurrent.CopyOnWriteArrayList
import java.util.concurrent.atomic.AtomicInteger

/** Hosted Binder cache/death adapter. Native surfaces/processes remain simulated. */
private class DisplayNativeContext : ContextWrapper(RuntimeEnvironment.getApplication()) {
    class Native : LifecycleRun {
        val entered = CompletableDeferred<Unit>()
        val retiring = CompletableDeferred<Unit>()
        val release = CompletableDeferred<Unit>()
        val retired = CompletableDeferred<Unit>()
        override suspend fun execute() { entered.complete(Unit); awaitCancellation() }
        override fun cancel() {}
        override suspend fun retire() { retiring.complete(Unit); release.await(); retired.complete(Unit) }
    }
    class Endpoint(private val target: IBinder, val native: Native) : Binder() {
        private val deaths = CopyOnWriteArrayList<IBinder.DeathRecipient>()
        @Volatile private var alive = true
        override fun isBinderAlive(): Boolean = alive
        override fun linkToDeath(recipient: IBinder.DeathRecipient, flags: Int) { deaths.add(recipient) }
        override fun unlinkToDeath(recipient: IBinder.DeathRecipient, flags: Int): Boolean = deaths.remove(recipient)
        override fun onTransact(code: Int, data: Parcel, reply: Parcel?, flags: Int): Boolean = target.transact(code, data, reply, flags)
        fun die() { alive = false; deaths.forEach { it.binderDied() } }
    }
    val endpoints = CopyOnWriteArrayList<Endpoint>()
    val binds = AtomicInteger()
    val unbinds = AtomicInteger()
    private class Slot(val controller: ServiceController<WorkerProbeService>, val native: Native)
    private val services = List(2) { createSlot() }
    override fun getApplicationContext(): Context = this
    @Synchronized override fun bindService(intent: Intent, connection: ServiceConnection, flags: Int): Boolean {
        val endpoint = endpoints.lastOrNull()?.takeIf { it.isBinderAlive } ?: open(intent)
        binds.incrementAndGet()
        connection.onServiceConnected(intent.component, endpoint)
        return true
    }
    private fun createSlot(): Slot {
        val controller = Robolectric.buildService(WorkerProbeService::class.java).create()
        val native = Native()
        controller.get().factory = { _, _, _ -> native }
        return Slot(controller, native)
    }
    private fun open(intent: Intent): Endpoint {
        val slot = services[endpoints.size]
        return Endpoint(slot.controller.get().onBind(intent), slot.native).also { endpoints.add(it) }
    }
    override fun unbindService(connection: ServiceConnection) { unbinds.incrementAndGet() }
    suspend fun close() {
        services.forEach { it.native.release.complete(Unit) }
        endpoints.forEach { it.die() }
        while (unbinds.get() != binds.get()) delay(1)
        services.forEach { it.controller.destroy() }
        services.forEach { ReflectionHelpers.getField<CoroutineScope>(it.controller.get(), "scope").coroutineContext[Job]!!.join() }
    }
}

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [27, 34])
class DisplayReplacementNativeTest {
    private fun view(surface: Surface) = object : SurfaceView(RuntimeEnvironment.getApplication()) {
        override fun getHolder(): SurfaceHolder = Proxy.newProxyInstance(SurfaceHolder::class.java.classLoader,
            arrayOf(SurfaceHolder::class.java)) { _, method, _ -> if (method.name == "getSurface") surface else null } as SurfaceHolder
    }
    private suspend fun until(condition: () -> Boolean) { while (!condition()) delay(1) }

    @Test fun t1416OriginalBinderDeathAndUnbindFenceReplacement() = runBlocking {
        val context = DisplayNativeContext()
        val scope = CoroutineScope(SupervisorJob() + Dispatchers.Unconfined)
        val texture = SurfaceTexture(0)
        val surface = Surface(texture)
        val workers = DisplayWorkerOwner(scope)
        fun receiver() = RemoteDisplay(RuntimeEnvironment.getApplication(), scope, workers = workers) { task, output, report ->
            AndroidWorkerRun(context, task, output, report, graceMillis = 60000)
        }.also { it.token = "a".repeat(64); it.setSurface(view(surface)) }
        val first = receiver()
        val next = receiver()
        try {
            first.start(); until { context.endpoints.isNotEmpty() }
            val original = context.endpoints.single()
            original.native.entered.await()
            first.stop(); original.native.retiring.await(); next.start()
            assertEquals("T1416 original binding still owns the singleton", 1, context.binds.get())
            original.native.release.complete(Unit); original.native.retired.await()
            assertEquals("T1416 local completion is not Binder death", 1, context.binds.get())
            original.die()
            until { context.endpoints.size == 2 }
            context.endpoints.last().native.entered.await()
            assertEquals(2, context.binds.get())
            assertEquals("T1416 original unbind precedes replacement", 1, context.unbinds.get())
            first.stop(); first.onSurfaceDestroyed()
            assertTrue(context.endpoints.last().isBinderAlive)
        } finally {
            first.stop(); next.stop(); context.close()
            scope.coroutineContext[Job]!!.cancelAndJoin(); surface.release(); texture.release()
        }
    }
}
