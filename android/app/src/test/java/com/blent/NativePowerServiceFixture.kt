package com.blent

import android.content.Intent
import java.util.concurrent.CopyOnWriteArrayList
import kotlinx.coroutines.*
import org.robolectric.RuntimeEnvironment
import org.robolectric.Shadows.shadowOf
import org.robolectric.android.controller.ServiceController

/** Executes real local native adapters under Robolectric. The injected process
 * wrapper joins actual local execution/retirement; it does not fabricate death.
 * Unknown/failing native ownership stays in original runs for fixture cleanup. */
internal class NativePowerServiceFixture {
    class Original(val run: LifecycleRun) {
        @Volatile var ready = false
        @Volatile var ended = false
    }
    private val app = RuntimeEnvironment.getApplication()
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Default)
    val originals = CopyOnWriteArrayList<Original>()
    val runtime = StreamingPowerRuntime(app, factory = PowerWorkerFactory { task, report ->
        lateinit var original: Original
        val run = LocalMediaRuns.create(app, task, null) { event ->
            report(event)
            if (event is WorkerEvent.WakeReady) original.ready = true
        }
        original = Original(run)
        originals.add(original)
        object : LifecycleRun {
            override suspend fun execute() {
                try { run.execute() }
                finally { original.ended = true }
            }
            override fun cancel() = run.cancel()
            override suspend fun retire() = run.retire()
        }
    }, scope = scope)
    val controller = ServiceController.of(StreamingService(runtime), Intent()).create()
    val service get() = controller.get()
    private var owner: DisplayPowerOwner? = null
    private var destroyed = false

    fun update(power: StreamingPower) {
        val original = owner
        if (original == null) owner = runtime.open(power) else runtime.publish(original, power)
        idle()
    }
    fun idle() = shadowOf(android.os.Looper.getMainLooper()).idle()
    suspend fun until(predicate: () -> Boolean) {
        withTimeout(4000) { while (!predicate()) { idle(); delay(1) } }
        idle()
    }
    suspend fun ready(count: Int) = until { originals.size >= count && originals[count - 1].ready }
    suspend fun ended(count: Int) = until { originals.size >= count && originals[count - 1].ended }
    fun destroy() {
        if (destroyed) return
        destroyed = true
        controller.destroy(); idle()
    }
    suspend fun close() {
        owner?.let(runtime::close)
        destroy()
        try {
            until { scope.coroutineContext[Job]!!.children.none() }
            originals.forEach { it.run.retire() }
        } finally { scope.cancel() }
    }
}
