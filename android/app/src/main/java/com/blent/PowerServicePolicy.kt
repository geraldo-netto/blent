package com.blent

/** Actual receiver's Wi-Fi grace. Readiness is an original API acquisition
 * observation, never native retirement evidence. Repeated idle preserves due time. */
internal class PowerServicePolicy(
    private val worker: PowerWorkerScope,
    private val access: DisplayPowerView,
    private val timer: WifiIdleTimer,
    private val schedule: PowerSchedule,
    private val lost: (DisplayPowerOwner, Exception?) -> Unit,
) {
    private class Idle(val deadline: Long, var request: DisplayPowerRequest)
    private var idle: Idle? = null

    fun apply(request: DisplayPowerRequest): Boolean {
        val power = checkNotNull(request.power)
        return when (power.wifiDemand) {
            WifiWakeDemand.HOLD -> immediate(request, true)
            WifiWakeDemand.RELEASE -> immediate(request, false)
            WifiWakeDemand.IDLE_GRACE -> grace(request)
        }
    }
    private fun immediate(request: DisplayPowerRequest, wifi: Boolean): Boolean {
        idle = null
        timer.clear(this)
        return worker.update(request, wifi, schedule.now())
    }
    private fun grace(request: DisplayPowerRequest): Boolean {
        val original = idle ?: begin(request) ?: return worker.update(request, false, schedule.now())
        original.request = request
        val now = schedule.now()
        if (now >= original.deadline) {
            timer.clear(this)
            return worker.update(request, false, now)
        }
        if (!worker.update(request, true, now)) return false
        timer.set(WifiIdleTarget(this, original.deadline, { current(original) }, { release(original) }))
        return true
    }
    private fun begin(request: DisplayPowerRequest): Idle? {
        if (!worker.wifiReady) return null
        val now = schedule.now()
        require(now >= 0)
        return Idle(Math.addExact(now, WIFI_IDLE_GRACE_MILLIS), request).also { idle = it }
    }
    private fun current(original: Idle): Boolean = idle === original && worker.alive && access.current(original.request)
    private fun release(original: Idle) {
        if (!current(original)) return
        try { if (!worker.update(original.request, false, schedule.now())) lost(worker.owner, null) }
        catch (failure: Exception) { lost(worker.owner, failure) }
    }
    fun close() {
        idle = null
        timer.clear(this)
        worker.close()
    }
}
