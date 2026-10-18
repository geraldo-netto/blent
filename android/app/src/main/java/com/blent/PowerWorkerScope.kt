package com.blent

import java.util.concurrent.atomic.AtomicBoolean
import kotlinx.coroutines.flow.MutableStateFlow

/** One original foreground/service execution view. Actual native ownership is
 * retained by the shared retirement lane, even after this view is cut off. */
internal class PowerWorkerScope(
    initial: DisplayPowerRequest,
    private val access: DisplayPowerView,
    private val lane: ResourceLane,
    private val factory: PowerWorkerFactory,
    private val postUi: (() -> Unit) -> Unit,
    private val lost: (DisplayPowerOwner, Exception?) -> Unit,
) {
    val owner = initial.owner
    private val cutoff = AtomicBoolean()
    private val views = MutableStateFlow<PowerWorkerView?>(PowerWorkerView(initial, null, Any()))
    private var readyStamp: Any? = null
    private var startedMillis = 0L
    val alive get() = !cutoff.get()
    val wifiReady get() = views.value?.let { it.stamp === readyStamp && it.task?.wifi == true } == true

    fun update(request: DisplayPowerRequest, wifi: Boolean, now: Long): Boolean {
        if (!alive) return false
        require(request.owner === owner && now >= startedMillis)
        val before = views.value ?: return false
        val desired = desired(request, wifi)
        val stamp = nextStamp(before, desired, now)
        views.value = PowerWorkerView(request, desired, stamp)
        if (stamp === before.stamp) return true
        readyStamp = null
        if (desired == null) lane.stop() else start(desired, stamp, now)
        return true
    }
    private fun desired(request: DisplayPowerRequest, wifi: Boolean): MediaTask.Wake? {
        val cpu = checkNotNull(request.power).needsCpu
        return if (cpu || wifi) MediaTask.Wake(cpu, wifi) else null
    }
    private fun nextStamp(before: PowerWorkerView, desired: MediaTask.Wake?, now: Long): Any {
        val renew = desired?.cpu == true && now - startedMillis >= CPU_WAKE_MILLIS
        return if (before.task == desired && !renew) before.stamp else Any()
    }
    private fun start(task: MediaTask.Wake, stamp: Any, now: Long) {
        startedMillis = now
        lane.replace({
            ScopedPowerRun(views, stamp, cutoff, access) {
                factory.create(task) { event -> postUi { receive(stamp, event) } }
            }
        }, { failure -> ended(stamp, failure) })
    }
    private fun receive(stamp: Any, event: WorkerEvent) {
        val view = views.value ?: return
        if (!alive || view.stamp !== stamp || !access.owns(owner)) return
        if (event !is WorkerEvent.WakeReady || view.task != MediaTask.Wake(event.cpu, event.wifi)) {
            ended(stamp, IllegalStateException("Invalid original power worker readiness"))
            return
        }
        readyStamp = stamp
    }
    private fun ended(stamp: Any, failure: Exception?) {
        val view = views.value ?: return
        if (view.stamp !== stamp || !cutoff.compareAndSet(false, true)) return
        postUi { lost(owner, failure) }
    }
    fun close() {
        cutoff.set(true)
        if (views.value == null) return
        views.value = null
        lane.stop()
    }
}
