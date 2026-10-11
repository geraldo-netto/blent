package com.blent

import kotlinx.coroutines.*

/** Safe proof for one original request; failure/timeout never produces proof. */
internal enum class ResourceRetirement { NOT_STARTED, RETIRED }

/** One retiring owner and one latest request. A deadline reports a stall;
 * it never transfers ownership or authorizes replacement.
 * Factories return lazy wrappers without native acquisition; opening happens
 * only inside execute.
 * Execution-ended observers run before retirement and cannot prove release.
 * Separate retirement observers run after the job and actual native cleanup,
 * or after an unopened request is discarded. Observer failures cannot skip
 * cleanup or authorize replacement; fatal VM errors are not consumed. */
internal class ResourceLane(
    private val scope: CoroutineScope,
    private val report: (String) -> Unit,
    private val deadlineMillis: Long = 5_000,
) {
    init { require(deadlineMillis in 1..60_000) }
    private class Request(
        val create: () -> LifecycleRun, val complete: (Exception?) -> Unit,
        val onRetired: (ResourceRetirement) -> Unit,
    ) { var proofPublished = false }
    private class Active(val request: Request) {
        lateinit var job: Job
        @Volatile var run: LifecycleRun? = null
        var stopping = false
        var deadline: Job? = null
        var failure: Exception? = null
        var notified = false
        var retirement: ResourceRetirement? = null
    }
    private val guard = RetirementGuard()
    private var current: Active? = null
    private var pending: Request? = null
    @Volatile var stalled = false
        private set
    val error: Exception? get() = guard.error

    fun checkAvailable() = guard.checkAvailable()

    @Synchronized fun replace(create: () -> LifecycleRun, complete: (Exception?) -> Unit, retired: (ResourceRetirement) -> Unit = {}) {
        checkAvailable()
        val displaced = pending
        pending = Request(create, complete, retired)
        val owner = current
        if (owner == null) admit() else requestStop(owner)
        displaced?.let { notifyRetired(it, ResourceRetirement.NOT_STARTED) }
    }

    @Synchronized fun stop() {
        val displaced = pending
        pending = null
        current?.let { requestStop(it) }
        displaced?.let { notifyRetired(it, ResourceRetirement.NOT_STARTED) }
    }

    private fun admit() {
        if (current != null) return
        val request = pending ?: return
        pending = null
        if (!scope.isActive || error != null) {
            notifyRetired(request, ResourceRetirement.NOT_STARTED)
            return
        }
        val owner = Active(request)
        current = owner
        owner.job = scope.launch(start = CoroutineStart.LAZY) { execute(owner) }
        owner.job.invokeOnCompletion { complete(owner) }
        owner.job.start()
    }

    private suspend fun execute(owner: Active) {
        try {
            val run = owner.request.create()
            owner.run = run
            run.execute()
        } catch (failure: Exception) {
            if (failure !is CancellationException) owner.failure = failure
        } finally {
            try { notifyEnded(owner); beginDeadline(owner) }
            finally { retire(owner) }
        }
    }

    private suspend fun retire(owner: Active) {
        val run = owner.run
        if (run == null) owner.retirement = ResourceRetirement.NOT_STARTED
        else {
            val failure = withContext(NonCancellable) { guard.retire(run) }
            if (failure == null) owner.retirement = ResourceRetirement.RETIRED
            else status("Native cleanup incomplete: ${failure.message}")
        }
    }

    private fun requestStop(owner: Active) {
        if (stalled) status("Native cleanup stalled; replacement waiting.")
        if (owner.stopping) return
        owner.stopping = true
        beginDeadline(owner)
        cleanupAll({ owner.run?.cancel() }, { owner.job.cancel() }).firstOrNull()?.let {
            status("Cancellation incomplete: ${it.message}")
        }
    }

    @Synchronized private fun beginDeadline(owner: Active) {
        if (current !== owner || owner.deadline != null) return
        owner.deadline = CoroutineScope(Dispatchers.Default).launch {
            delay(deadlineMillis)
            markStalled(owner)
        }
    }

    @Synchronized private fun markStalled(owner: Active) {
        if (current !== owner) return
        stalled = true
        status("Native cleanup stalled; replacement waiting.")
    }

    private fun notifyEnded(owner: Active) {
        if (owner.notified) return
        owner.notified = true
        notify("Execution-ended", { owner.request.complete(owner.failure) })
    }

    private fun notifyRetired(request: Request, proof: ResourceRetirement) {
        if (request.proofPublished) return
        request.proofPublished = true
        notify("Retirement", { request.onRetired(proof) })
    }

    private fun notify(label: String, callback: () -> Unit) {
        try { callback() }
        catch (failure: Exception) { status("$label callback failed: ${failure.message}") }
    }

    private fun status(text: String) {
        try { report(text) } catch (_: Exception) { /* Observers cannot own native cleanup. */ }
    }

    @Synchronized private fun complete(owner: Active) {
        if (current !== owner) return
        owner.deadline?.cancel()
        notifyEnded(owner)
        val proof = owner.retirement ?: if (owner.run == null) ResourceRetirement.NOT_STARTED else null
        if (proof == null && error == null) {
            // A failure before the guard was entered still cannot release a run.
            stalled = true
            status("Native cleanup unconfirmed; replacement waiting.")
            return
        }
        proof?.let { notifyRetired(owner.request, it) }
        // Keep the original owner installed through callbacks. Reentrant replace
        // can update the latest request but cannot admit a second native owner.
        current = null
        val recovered = stalled
        stalled = false
        if (error != null) {
            val displaced = pending
            pending = null
            displaced?.let { notifyRetired(it, ResourceRetirement.NOT_STARTED) }
            status("Native cleanup incomplete: ${error?.message}")
        } else {
            if (recovered) status("Native cleanup complete.")
            admit()
        }
    }
}
