package com.blent

/** One original queued admission plus one latest target across service owners.
 * Unknown scheduling/cancellation keeps admission but withdraws grace demand. */
internal class WifiIdleTimer(private val schedule: PowerSchedule, private val report: (Exception) -> Unit) {
    private inner class Pending {
        val action = Runnable { received(this) }
        var uncertain = false
    }
    private var target: WifiIdleTarget? = null
    private var queued: Pending? = null

    fun set(value: WifiIdleTarget) {
        val now = now()
        require(value.deadline >= 0 && value.deadline - now <= WIFI_IDLE_GRACE_MILLIS)
        target = value
        arm()
    }
    fun clear(owner: Any) {
        if (target?.owner !== owner) return
        target = null
        val original = queued ?: return
        try { schedule.cancel(original.action); if (queued === original) queued = null }
        catch (failure: Exception) { original.uncertain = true; failed(failure) }
    }
    private fun now(): Long = schedule.now().also { require(it >= 0) }
    private fun arm() {
        val latest = target ?: return
        val pending = queued
        if (pending?.uncertain == true) { fire(latest); return }
        if (pending != null) return
        val delay = latest.deadline - now()
        if (delay <= 0) fire(latest) else post(latest, delay)
    }
    private fun post(goal: WifiIdleTarget, delay: Long) {
        val original = Pending()
        queued = original
        try {
            if (!schedule.post(delay, original.action)) reject(original, goal)
        } catch (failure: Exception) {
            if (queued === original) { original.uncertain = true; fireCurrent() }
            failed(failure)
        }
    }
    private fun reject(original: Pending, goal: WifiIdleTarget) {
        if (queued !== original) return
        queued = null
        if (target === goal) fire(goal) else arm()
    }
    private fun received(original: Pending) {
        if (queued !== original) return
        queued = null
        try { arm() }
        catch (failure: Exception) { failed(failure); fireCurrent() }
    }
    private fun fireCurrent() { target?.let(::fire) }
    private fun fire(goal: WifiIdleTarget) {
        if (target !== goal) return
        target = null
        if (goal.current()) goal.release()
    }
    private fun failed(failure: Exception) {
        try { report(failure) }
        catch (report: Exception) { if (report !== failure) failure.addSuppressed(report) }
    }
}
