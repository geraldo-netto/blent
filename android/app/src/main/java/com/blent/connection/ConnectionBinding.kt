package com.blent.connection

import com.blent.LifecycleRun
import com.blent.ResourceLane
import kotlinx.coroutines.CoroutineScope

/** Process-owned UI controller. Native work belongs to the injected worker;
 * The factory must be lazy: no native work before execute.
 * ResourceLane retains ownership through execution AND actual retirement.
 * Public actions serialize independently from event/reducer callbacks, avoiding
 * lock inversion with the lane's factory, completion and deadline callbacks. */
internal class ConnectionBinding(
    scope: CoroutineScope,
    private val create: (ConnectionWorkerSpec, (ConnectionWorkerEvent) -> Unit) -> ConnectionWorker,
    cleanupNoticeMs: Long = 5000,
) {
    private class Attempt(val token: Any) { @Volatile var worker: ConnectionWorker? = null }
    private val presentation = ConnectionPresentation()
    val state = presentation.state
    private val lane = ResourceLane(scope, ::cleanup, cleanupNoticeMs)
    @Volatile private var current: Attempt? = null

    private fun cleanup(text: String) {
        if (lane.error != null) current?.let { presentation.ended(it.token, text) }
        presentation.cleanup(text)
    }

    @Synchronized fun connect(spec: ConnectionWorkerSpec) {
        try { lane.checkAvailable() }
        catch (failure: Exception) { presentation.cleanup(failure.message ?: "USB cleanup incomplete."); return }
        val attempt = Attempt(presentation.begin())
        current = attempt
        lane.replace({ open(attempt, spec) }, {
            presentation.ended(attempt.token, it?.message)
            if (attempt.worker == null) presentation.retired(attempt.token)
        })
    }

    private fun open(attempt: Attempt, spec: ConnectionWorkerSpec): LifecycleRun {
        val worker = create(spec) { presentation.event(attempt.token, it) }
        attempt.worker = worker
        presentation.started(attempt.token)
        return object : LifecycleRun {
            override suspend fun execute() = worker.execute()
            override fun cancel() = worker.cancel()
            override suspend fun retire() {
                worker.retire()
                presentation.retired(attempt.token)
            }
        }
    }

    @Synchronized fun disconnect() {
        current?.let { presentation.stop(it.token) }
        lane.stop()
    }

    @Synchronized fun decide(choice: ConnectionChoice, allow: Boolean, remember: Boolean): Boolean {
        val attempt = current ?: return false
        val command = presentation.decide(attempt.token, choice, allow, remember) ?: return false
        return send(attempt, command)
    }

    @Synchronized fun revoke(choice: ConnectionChoice): Boolean {
        val attempt = current ?: return false
        val command = presentation.revoke(attempt.token, choice) ?: return false
        return send(attempt, command)
    }

    private fun send(attempt: Attempt, command: ConnectionWorkerCommand): Boolean = try {
        check(attempt.worker?.command(command) == true) { "USB command rejected; reconnect to try again." }
        true
    } catch (failure: Exception) {
        presentation.ended(attempt.token, failure.message ?: "USB command failed.")
        lane.stop()
        false
    }
}
