package com.blent

import android.content.ComponentName
import android.content.ContextWrapper
import android.content.Intent
import java.util.concurrent.ConcurrentLinkedQueue
import kotlinx.coroutines.*
import org.junit.Assert.*
import org.robolectric.RuntimeEnvironment

/** Portable native-process model: original retire gate, independent task/event
 * list and explicit settlement. No physical process or wake handle is created. */
internal class PowerRuntimeFixture(hold: Boolean = false, ready: Boolean = true) {
    class App : ContextWrapper(RuntimeEnvironment.getApplication()) {
        val starts = ArrayList<Intent>()
        var denied = false
        var ambiguous = false
        var missing = false
        override fun getApplicationContext() = this
        override fun startForegroundService(intent: Intent): ComponentName? {
            if (denied) throw SecurityException("T1060 definitely not admitted")
            starts.add(intent)
            if (ambiguous) throw IllegalStateException("T1060 accepted then failed")
            return if (missing) null else ComponentName(this, StreamingService::class.java)
        }
    }
    class Schedule : PowerSchedule {
        data class Entry(val due: Long, val action: Runnable)
        var time = 0L
        var refuse = false
        var postFailure = false
        var cancelFailure = false
        val entries = ArrayList<Entry>()
        override fun now() = time
        override fun post(delayMillis: Long, action: Runnable): Boolean {
            if (refuse) return false
            entries.add(Entry(Math.addExact(time, delayMillis), action))
            if (postFailure) error("T1060 queued timer then failed")
            return true
        }
        override fun cancel(action: Runnable) {
            if (cancelFailure) error("T1060 timer cutoff unknown")
            entries.removeAll { it.action === action }
        }
        fun due() {
            val ready = entries.filter { it.due <= time }
            entries.removeAll(ready.toSet())
            ready.forEach { it.action.run() }
        }
    }
    val app = App()
    val schedule = Schedule()
    val scope = CoroutineScope(SupervisorJob() + Dispatchers.Unconfined)
    val ui = ConcurrentLinkedQueue<() -> Unit>()
    val native = ArrayList<MediaTask.Wake>()
    val events = ArrayList<(WorkerEvent) -> Unit>()
    val release = CompletableDeferred<Unit>().apply { if (!hold) complete(Unit) }
    var canceled = 0
    var settled = 0
    val runtime = StreamingPowerRuntime(app, schedule, PowerWorkerFactory { task, report ->
        native.add(task); events.add(report)
        object : LifecycleRun {
            override suspend fun execute() { if (ready) report(WorkerEvent.WakeReady(task.cpu, task.wifi)); awaitCancellation() }
            override fun cancel() { canceled++ }
            override suspend fun retire() { release.await(); settled++ }
        }
    }, { ui.add(it) }, scope = scope)
    val notices = ConcurrentLinkedQueue<DisplayPowerNotice>()
    var stops = 0
    var postRefused = false
    var postThrows = false
    val port = object : PowerServicePort {
        override fun post(notice: DisplayPowerNotice): Boolean {
            if (postRefused) return false
            notices.add(notice)
            if (postThrows) error("T1060 queued native notice then threw")
            return true
        }
        override fun stop() { stops++ }
    }
    fun attachFirst() {
        assertTrue(runtime.attach(port))
        val first = checkNotNull(PowerServiceNotice.read(app.starts.last()))
        runtime.receive(port, first, first.matches(app.starts.last()))
        drain()
    }
    fun drain() {
        while (true) {
            val notice = notices.poll()
            if (notice != null) runtime.receive(port, PowerServiceNotice(runtime, notice))
            else { val action = ui.poll() ?: return; action() }
        }
    }
    suspend fun finish(owner: DisplayPowerOwner) {
        release.complete(Unit)
        runtime.close(owner); drain(); runtime.detach(port); drain()
        withTimeout(2000) { while (settled != native.size) yield() }
        scope.cancel()
    }
}
