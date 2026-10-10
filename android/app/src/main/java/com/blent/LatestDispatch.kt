package com.blent

import kotlinx.coroutines.*

/** Replaceable observations only: one scheduled drain and one latest action.
 * Ownership completion and ordered data must use their own contracts. */
internal class LatestDispatch(private val scope: CoroutineScope, private val reportFailure: (Exception) -> Unit) {
    private var owner: Any? = null
    private var pending: (() -> Unit)? = null
    @Volatile var failure: Exception? = null
        private set

    @OptIn(ExperimentalCoroutinesApi::class)
    fun submit(action: () -> Unit) {
        val token = synchronized(this) {
            pending = action
            if (owner != null) return
            Any().also { owner = it }
        }
        // ATOMIC guarantees finally clears admission even if cancellation wins
        // before the dispatcher starts this job. No native work happens here.
        scope.launch(start = CoroutineStart.ATOMIC) {
            try { drain(token) }
            catch (error: Exception) { if (error !is CancellationException) failed(error) }
            finally { finish(token) }
        }
    }
    private suspend fun drain(token: Any) {
        while (true) {
            currentCoroutineContext().ensureActive()
            val next = take(token) ?: return
            next()
            yield() // A continuous producer must not monopolize the UI owner.
        }
    }
    @Synchronized private fun take(token: Any): (() -> Unit)? {
        if (owner !== token) return null
        val next = pending
        pending = null
        if (next == null) owner = null
        return next
    }
    @Synchronized private fun finish(token: Any) {
        if (owner !== token) return
        owner = null; pending = null
    }
    private fun failed(error: Exception) {
        failure = error
        try { reportFailure(error) } catch (report: Exception) { if (report !== error) error.addSuppressed(report) }
    }
}
