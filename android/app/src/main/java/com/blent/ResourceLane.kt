package com.blent

import kotlinx.coroutines.*

/** One retiring owner and one latest request. A deadline reports a stall;
 * it never transfers ownership or authorizes replacement. */
internal class ResourceLane(
    private val scope: CoroutineScope,
    private val report: (String) -> Unit,
    private val deadlineMillis: Long = 5_000,
) {
    init { require(deadlineMillis in 1..60_000) }
    private data class Request(val create: () -> LifecycleRun, val complete: (Exception?) -> Unit)
    private class Active(val request: Request) {
        lateinit var job: Job
        @Volatile var run: LifecycleRun? = null
        var stopping = false
        var deadline: Job? = null
        var failure: Exception? = null
        var notified = false
    }
    private val guard = RetirementGuard()
    private var current: Active? = null
    private var pending: Request? = null
    @Volatile var stalled = false
        private set
    val error: Exception? get() = guard.error

    fun checkAvailable() = guard.checkAvailable()

    @Synchronized fun replace(create: () -> LifecycleRun, complete: (Exception?) -> Unit) {
        checkAvailable()
        pending = Request(create, complete)
        val owner = current
        if (owner == null) admit() else requestStop(owner)
    }

    @Synchronized fun stop() {
        pending = null
        current?.let { requestStop(it) }
    }

    private fun admit() {
        val request = pending ?: return
        pending = null
        if (!scope.isActive || error != null) return
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
            notifyEnded(owner)
            beginDeadline(owner)
            owner.run?.let { run ->
                val failure = withContext(NonCancellable) { guard.retire(run) }
                if (failure != null) report("Native cleanup incomplete: ${failure.message}")
            }
        }
    }

    private fun requestStop(owner: Active) {
        if (stalled) report("Native cleanup stalled; replacement waiting.")
        if (owner.stopping) return
        owner.stopping = true
        beginDeadline(owner)
        cleanupAll({ owner.run?.cancel() }, { owner.job.cancel() }).firstOrNull()?.let {
            report("Cancellation incomplete: ${it.message}")
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
        report("Native cleanup stalled; replacement waiting.")
    }

    private fun notifyEnded(owner: Active) {
        if (owner.notified) return
        owner.notified = true
        owner.request.complete(owner.failure)
    }

    @Synchronized private fun complete(owner: Active) {
        if (current !== owner) return
        owner.deadline?.cancel()
        current = null
        val recovered = stalled
        stalled = false
        notifyEnded(owner)
        if (error != null) {
            pending = null
            report("Native cleanup incomplete: ${error?.message}")
        } else {
            if (recovered) report("Native cleanup complete.")
            admit()
        }
    }
}

