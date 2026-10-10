package com.blent

import android.content.*
import android.os.*
import kotlinx.coroutines.*
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.RuntimeEnvironment
import org.robolectric.annotation.Config
import java.util.concurrent.CopyOnWriteArrayList

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [27, 34])
class AndroidWorkerRunTest {
    private val task = MediaTask.Metadata(640, 480, 30)
    private class Endpoint : Binder() {
        val deaths = CopyOnWriteArrayList<IBinder.DeathRecipient>()
        val commands = CopyOnWriteArrayList<Int>()
        var revision = 0L
        var callback: IBinder? = null
        var killWorks = true
        var failLink = false
        var failStop = false
        var start: (Endpoint) -> Unit = { it.emit(WorkerEvent.Result("ok")); it.emit(WorkerEvent.Ended(null)) }
        override fun linkToDeath(recipient: IBinder.DeathRecipient, flags: Int) {
            if (failLink) throw RemoteException("dead binding")
            deaths.add(recipient)
        }
        override fun unlinkToDeath(recipient: IBinder.DeathRecipient, flags: Int) = deaths.remove(recipient)
        override fun onTransact(code: Int, data: Parcel, reply: Parcel?, flags: Int): Boolean {
            WorkerIpc.validate(data, WorkerIpc.REQUEST)
            revision = data.readLong(); commands.add(code)
            when (code) {
                WorkerIpc.START -> { MediaTaskJson.decode(requireNotNull(data.readString())); callback = data.readStrongBinder(); start(this) }
                WorkerIpc.STOP -> if (failStop) throw RemoteException("stop failed")
                WorkerIpc.EXIT -> if (killWorks) deaths.toList().forEach { it.binderDied() }
            }
            return true
        }
        fun emit(event: WorkerEvent, generation: Long = revision) = raw(WorkerEventJson.encode(event), generation)
        fun raw(text: String, generation: Long = revision) = WorkerIpc.event(checkNotNull(callback), generation, text)
    }
    private class Context(val endpoint: Endpoint) : ContextWrapper(RuntimeEnvironment.getApplication()) {
        var flags = 0
        var binds = 0
        var unbinds = 0
        var accept = true
        var error = false
        var nullBinding = false
        var connect = true
        var unbindError = false
        lateinit var connection: ServiceConnection
        lateinit var component: ComponentName
        override fun getApplicationContext() = this
        override fun bindService(intent: Intent, connection: ServiceConnection, flags: Int): Boolean {
            binds++; this.flags = flags; this.connection = connection; component = intent.component!!
            if (error) throw SecurityException("bind denied")
            if (!accept) return false
            if (nullBinding) connection.onNullBinding(component)
            else if (connect) connection.onServiceConnected(component, endpoint)
            return true
        }
        override fun unbindService(connection: ServiceConnection) {
            unbinds++
            if (unbindError) throw IllegalArgumentException("unbind failed")
        }
    }
    @Test fun t809_workerResultRequiresGenerationAndConfirmedDeathBeforeRetirement() = runBlocking {
        val endpoint = Endpoint().apply { start = {
            it.emit(WorkerEvent.Status("stale"), it.revision - 1)
            it.emit(WorkerEvent.Result("ok")); it.emit(WorkerEvent.Ended(null))
        } }
        val context = Context(endpoint)
        val events = mutableListOf<WorkerEvent>()
        val run = AndroidWorkerRun(context, task, report = events::add, graceMillis = 1, exitMillis = 1000)
        run.execute(); run.retire()
        assertEquals(listOf(WorkerEvent.Result("ok")), events)
        assertTrue(endpoint.commands.containsAll(listOf(WorkerIpc.START, WorkerIpc.STOP, WorkerIpc.EXIT)))
        assertEquals(1, context.unbinds); assertTrue(endpoint.deaths.isEmpty())
        assertTrue(context.flags and android.content.Context.BIND_AUTO_CREATE != 0)
        if (Build.VERSION.SDK_INT >= 30) assertTrue(context.flags and android.content.Context.BIND_INCLUDE_CAPABILITIES != 0)
        run.retire(); assertEquals(1, context.unbinds)
    }
    @Test fun t809_stopAndDeathAreResponsiveWithoutNativeCompletion() = runBlocking {
        val endpoint = Endpoint().apply { start = {} }
        val context = Context(endpoint)
        val run = AndroidWorkerRun(context, task, graceMillis = 5, exitMillis = 1000)
        val active = async(Dispatchers.Default) { runCatching { run.execute() } }
        withTimeout(1000) { while (endpoint.callback == null) delay(1) }
        run.cancel(); run.retire()
        assertTrue(withTimeout(1000) { active.await() }.isFailure)
        val dead = Endpoint().apply { start = { deaths.toList().forEach { it.binderDied() } } }
        val killed = AndroidWorkerRun(Context(dead), task, graceMillis = 1, exitMillis = 1000)
        assertTrue(runCatching { killed.execute() }.isFailure)
        killed.retire()
    }
    @Test fun t809_failedExitRemainsUnconfirmedAndTransportFailuresRetire() = runBlocking {
        val endpoint = Endpoint().apply { killWorks = false; failStop = true }
        val context = Context(endpoint)
        val run = AndroidWorkerRun(context, task, graceMillis = 1, exitMillis = 30)
        run.execute()
        assertTrue(runCatching { run.retire() }.exceptionOrNull() is TimeoutCancellationException)
        assertEquals(1, context.unbinds)
        val error = Endpoint().apply { start = { it.emit(WorkerEvent.Ended("native failed")) } }
        val failed = AndroidWorkerRun(Context(error), task, graceMillis = 1)
        assertEquals("native failed", runCatching { failed.execute() }.exceptionOrNull()?.message)
        failed.retire()
    }
    @Test fun t809_bindFailureCancellationAndLateConnectionsNeverStartNativeWork() = runBlocking {
        for (mode in 0..4) {
            val endpoint = Endpoint()
            val context = Context(endpoint).apply {
                accept = mode != 0; error = mode == 1; nullBinding = mode == 2
                endpoint.failLink = mode == 3; connect = mode != 4
            }
            val run = AndroidWorkerRun(context, task, graceMillis = 1, exitMillis = 100)
            if (mode == 4) {
                val pending = async(Dispatchers.Default) { runCatching { run.execute() } }
                withTimeout(1000) { while (context.binds == 0) delay(1) }
                run.cancel(); pending.cancelAndJoin(); run.retire()
                context.connection.onServiceConnected(context.component, endpoint)
            } else {
                assertTrue(runCatching { run.execute() }.isFailure)
                run.retire()
            }
            assertFalse(endpoint.commands.contains(WorkerIpc.START))
        }
        val context = Context(Endpoint())
        val cancelled = AndroidWorkerRun(context, task)
        cancelled.cancel()
        assertTrue(runCatching { cancelled.execute() }.isFailure)
        cancelled.retire(); assertEquals(0, context.binds)
    }
    @Test fun t809_invalidAndOverloadedEventsCannotGrowTheQueue() = runBlocking {
        for (bad in listOf("{\"x\":" + "[".repeat(100) + "0" + "]".repeat(100) + "}", " ".repeat(262145))) {
            val endpoint = Endpoint().apply { start = { it.raw(bad) } }
            val run = AndroidWorkerRun(Context(endpoint), task, graceMillis = 1)
            assertTrue(runCatching { withTimeout(1000) { run.execute() } }.isFailure)
            run.retire()
        }
        val endpoint = Endpoint().apply { start = { remote -> repeat(40) { remote.emit(WorkerEvent.Status("queued")) } } }
        val run = AndroidWorkerRun(Context(endpoint), task, graceMillis = 1)
        assertTrue(runCatching { withTimeout(1000) { run.execute() } }.isFailure)
        run.retire()
    }
    @Test fun t809_frameworkDisconnectAndBindingDeathEndTheRun() = runBlocking {
        for (bindingDeath in listOf(false, true)) {
            val endpoint = Endpoint().apply { start = {} }
            val context = Context(endpoint)
            val run = AndroidWorkerRun(context, task, graceMillis = 1)
            val active = async(Dispatchers.Default) { runCatching { run.execute() } }
            withTimeout(1000) { while (endpoint.callback == null) delay(1) }
            if (bindingDeath) context.connection.onBindingDied(context.component)
            else context.connection.onServiceDisconnected(context.component)
            assertTrue(withTimeout(1000) { active.await() }.isFailure)
            run.retire(); assertEquals(1, context.unbinds)
        }
    }
    @Test fun t809_serviceClassMappingAndParcelBoundsAreExplicit() {
        val app = RuntimeEnvironment.getApplication()
        val token = "a".repeat(64)
        val tasks = listOf(task, MediaTask.Display(token, DecoderFormat("video/avc", 1, 1, 30)),
            MediaTask.Camera(CameraEndpoint(token, 1, 640, 480, 30, 1000), CameraLens.REAR, 0),
            MediaTask.Audio(AudioEndpoint(token, 1, 1, 1, 40, false), AudioPreferences()),
            MediaTask.Audio(AudioEndpoint(token, 1, 2, 1, 40, false), AudioPreferences()))
        assertEquals(5, tasks.map { AndroidWorkerRun(app, it).serviceClass(it) }.toSet().size)
        val parcel = Parcel.obtain()
        try { assertThrows(IllegalArgumentException::class.java) { WorkerIpc.validate(parcel, "missing") } }
        finally { parcel.recycle() }
        assertThrows(IllegalArgumentException::class.java) { WorkerIpc.command(Endpoint(), WorkerIpc.START, 1, "x".repeat(400000)) }
        assertThrows(IllegalArgumentException::class.java) { WorkerIpc.event(Endpoint(), 1, "x".repeat(400000)) }
    }
}
