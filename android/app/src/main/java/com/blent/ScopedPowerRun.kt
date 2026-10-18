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
        if (!awaitEligible()) return
        val original = create()
        owned = original
        currentCoroutineContext().ensureActive()
        if (!awaitEligible()) return
        currentCoroutineContext().ensureActive()
        original.execute()
    }
    private suspend fun awaitEligible(): Boolean {
        while (true) {
            val view = views.first { observable(it) }
            if (ended(view)) return false
            if (relevant(checkNotNull(view))) return true
        }
    }
    private fun observable(view: PowerWorkerView?): Boolean {
        if (ended(view)) return true
        return relevant(checkNotNull(view))
    }
    private fun ended(view: PowerWorkerView?): Boolean = stopped.get() || cutoff.get() || view == null || view.stamp !== stamp
    private fun relevant(view: PowerWorkerView): Boolean {
        val request = view.request
        if (!access.current(request)) return false
        return request.permitsPower() && access.current(request)
    }
    override fun cancel() { stopped.set(true); owned?.cancel() }
    override suspend fun retire() { cancel(); owned?.retire() }
}
