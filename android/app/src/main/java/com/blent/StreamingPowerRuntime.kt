package com.blent

import android.content.Context
import android.os.Handler
import android.os.Looper
import android.util.Log
import kotlinx.coroutines.*

/** Application lifetime owns the original queued handoff and retiring power
 * process. Foreground components never erase unknown native obligations. */
internal class StreamingPowerRuntime(
    private val app: Context,
    private val schedule: PowerSchedule = HandlerPowerSchedule(),
    private val factory: PowerWorkerFactory = PowerWorkerFactory { task, report -> AndroidWorkerRun(app, task, report = report) },
    private val postUi: (() -> Unit) -> Unit = { action -> check(Handler(Looper.getMainLooper()).post(action)) },
    private val channel: DisplayPowerChannel = DisplayPowerRequests(),
    scope: CoroutineScope = CoroutineScope(SupervisorJob() + Dispatchers.IO),
) {
    private class Session(val owner: DisplayPowerOwner, val port: PowerServicePort, val worker: PowerWorkerScope, val policy: PowerServicePolicy)
    private val lane = ResourceLane(scope, { Log.w("BlentPower", it) })
    private val timer = WifiIdleTimer(schedule) { Log.w("BlentPower", "Wi-Fi grace scheduling failed", it) }
    private var client: DisplayPowerOwner? = null
    private var service: PowerServicePort? = null
    private var session: Session? = null

    fun open(power: StreamingPower, permit: DisplayPowerPermit = FixedDisplayPowerPermit): DisplayPowerOwner {
        val admission = channel.open(power, permit)
        client = admission.owner
        cutoff()
        issue(admission.notice)
        return admission.owner
    }
    fun owns(owner: DisplayPowerOwner): Boolean = channel.owns(owner)
    fun pending(notice: DisplayPowerNotice): Boolean = channel.pending(notice)
    val hasOwner get() = client?.let(channel::owns) == true

    fun publish(owner: DisplayPowerOwner, power: StreamingPower, permit: DisplayPowerPermit = FixedDisplayPowerPermit): Boolean {
        if (!channel.owns(owner)) return false
        if (session?.let { it.owner === owner && !it.worker.alive } == true) { close(owner); return false }
        issue(channel.publish(owner, power, permit))
        return channel.owns(owner)
    }
    fun close(owner: DisplayPowerOwner) {
        if (!channel.owns(owner)) return
        issue(withdraw(owner))
        finishIfIdle()
    }
    private fun withdraw(owner: DisplayPowerOwner): DisplayPowerNotice? {
        if (!channel.owns(owner)) return null
        val notice = channel.close(owner)
        if (client === owner) client = null
        if (session?.owner === owner) cutoff()
        return notice
    }
    private fun cutoff() {
        val original = session ?: return
        session = null
        original.policy.close()
    }
    fun attach(port: PowerServicePort): Boolean {
        if (service != null && service !== port) return false
        service = port
        return true
    }
    fun detach(port: PowerServicePort) {
        if (service !== port) return
        service = null
        val original = session ?: return
        cutoff()
        issue(withdraw(original.owner))
    }
    fun receive(port: PowerServicePort, envelope: PowerServiceNotice, intentMatches: Boolean = true) {
        if (envelope.runtime !== this) return
        if (service !== port || !intentMatches) {
            rejectedReceipt(envelope.notice, IllegalStateException("Original foreground power delivery rejected"))
            return
        }
        val delivery = channel.receive(envelope.notice)
        if (!delivery.accepted) return
        delivery.request?.let { apply(port, it) }
        issue(delivery.next)
    }
    private fun apply(port: PowerServicePort, request: DisplayPowerRequest) {
        if (!channel.current(request)) return
        if (request.power == null) { if (session?.owner === request.owner) cutoff(); finishIfIdle(); return }
        try {
            if (!currentDesired(request)) return
            if (!activeSession(port, request).policy.apply(request)) lost(request.owner, null)
        } catch (failure: Exception) { lost(request.owner, failure) }
    }
    private fun currentDesired(request: DisplayPowerRequest): Boolean =
        client === request.owner && channel.owns(request.owner) && request.permitsPower() && channel.current(request)
    private fun activeSession(port: PowerServicePort, request: DisplayPowerRequest): Session {
        session?.let { if (it.owner === request.owner && it.port === port) return it }
        cutoff()
        val worker = PowerWorkerScope(request, channel, lane, factory, postUi, ::lost)
        return Session(request.owner, port, worker, PowerServicePolicy(worker, channel, timer, schedule, ::lost)).also { session = it }
    }
    private fun lost(owner: DisplayPowerOwner, failure: Exception?) {
        Log.w("BlentPower", "Original display wake owner ended", failure)
        close(owner)
    }
    fun rejectedReceipt(notice: DisplayPowerNotice, failure: Exception) {
        val delivery = channel.receive(notice)
        if (!delivery.accepted) return
        val stop = withdraw(notice.original.owner)
        Log.w("BlentPower", "Foreground power rejected", failure)
        issue(stop)
        issue(delivery.next)
        finishIfIdle()
    }
    /** Exact callback was fenced before execution; a still-current successor
     * gets a fresh notice rather than authority from the canceled callback. */
    fun canceled(notice: DisplayPowerNotice) {
        val delivery = channel.receive(notice)
        if (!delivery.accepted) return
        val request = delivery.request
        if (request?.power != null && channel.current(request)) issue(channel.publish(request.owner, request.power, request.permit))
        else issue(delivery.next)
        finishIfIdle()
    }
    private fun issue(notice: DisplayPowerNotice?) {
        if (notice == null) return
        val port = service
        if (port != null) { post(port, notice); return }
        if (notice.original.power == null) { canceled(notice); return }
        start(notice)
    }
    private fun post(port: PowerServicePort, notice: DisplayPowerNotice) {
        try {
            if (!port.post(notice)) rejectedReceipt(notice, IllegalStateException("Foreground power callback not queued"))
        } catch (failure: Exception) { unknown(notice, failure) }
    }
    private fun start(notice: DisplayPowerNotice) {
        try {
            if (app.startForegroundService(PowerServiceNotice(this, notice).intent(app)) == null)
                rejectedReceipt(notice, IllegalStateException("Foreground power component not admitted"))
        } catch (failure: SecurityException) { rejectedReceipt(notice, failure) }
        catch (failure: Exception) { unknown(notice, failure) }
    }
    private fun unknown(notice: DisplayPowerNotice, failure: Exception) {
        withdraw(notice.original.owner)
        Log.w("BlentPower", "Power handoff outcome unknown; original notice retained", failure)
        finishIfIdle()
    }
    private fun finishIfIdle() { if (!hasOwner) service?.stop() }

    companion object {
        fun forApp(context: Context): StreamingPowerRuntime =
            (context.applicationContext as BlentApplication).streamingPower
    }
}
