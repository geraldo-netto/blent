package com.blent

import java.util.concurrent.atomic.AtomicBoolean
import kotlinx.coroutines.*
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.first

/** Wait for current actual delivery before native admission. The containing lane
 * joins execute, including late lazy creators, before original retirement. */
internal class ScopedPowerRun(
    private val views: StateFlow<PowerWorkerView?>,
    private val stamp: Any,
    private val cutoff: AtomicBoolean,
    private val access: DisplayPowerView,
    private val create: () -> LifecycleRun,
) : LifecycleRun {
    private val stopped = AtomicBoolean()
    private val admitted = AtomicBoolean()
    @Volatile private var owned: LifecycleRun? = null

    override suspend fun execute() {
        currentCoroutineContext().ensureActive()
        check(admitted.compareAndSet(false, true)) { "Scoped power worker already executed" }
        val view = views.first { observable(it) }
        if (!eligible(view)) return
        val original = create()
        owned = original
        currentCoroutineContext().ensureActive()
        if (stopped.get()) original.cancel() else original.execute()
    }
    private fun observable(view: PowerWorkerView?): Boolean = stopped.get() || cutoff.get() || view == null || view.stamp !== stamp || access.current(view.request)
    private fun eligible(view: PowerWorkerView?): Boolean = !stopped.get() && !cutoff.get() && view != null && view.stamp === stamp && access.current(view.request)
    override fun cancel() { stopped.set(true); owned?.cancel() }
    override suspend fun retire() { cancel(); owned?.retire() }
}
