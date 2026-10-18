package com.blent

import kotlinx.coroutines.*
import java.util.concurrent.atomic.AtomicBoolean

/** Used only inside the private power process. STOP never performs native IO
 * on the Binder thread; original process death fences blocked/uncertain calls. */
internal class PowerWakeRun(
    private val task: MediaTask.Wake,
    createCpu: () -> WakeHandle,
    createWifi: () -> WakeHandle,
    private val ready: () -> Unit = {},
) : LifecycleRun {
    private val cpu = WakeLease(createCpu)
    private val wifi = WakeLease(createWifi)
    private val stopped = CompletableDeferred<Unit>()
    private val admitted = AtomicBoolean()
    override suspend fun execute() {
        currentCoroutineContext().ensureActive()
        check(admitted.compareAndSet(false, true)) { "Power worker already executed" }
        if (stopped.isCompleted) return
        if (task.cpu) cpu.request(true)
        currentCoroutineContext().ensureActive()
        if (stopped.isCompleted) return
        if (task.wifi) wifi.request(true)
        currentCoroutineContext().ensureActive()
        if (stopped.isCompleted) return
        ready()
        stopped.await()
    }
    override fun cancel() { stopped.complete(Unit) }
    override suspend fun retire() {
        cancel()
        val failures = cleanupAll(cpu::retire, wifi::retire)
        val first = failures.firstOrNull() ?: return
        failures.drop(1).forEach { if (it !== first) first.addSuppressed(it) }
        throw first
    }
}
