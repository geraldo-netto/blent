package com.blent

import kotlinx.coroutines.*

/** Replaceable observations only: one scheduled drain and one latest action.
 * Ownership completion and ordered data must use their own contracts. */
internal class LatestDispatch(private val scope: CoroutineScope, private val reportFailure: (Exception) -> Unit) {
    private var owner: Any? = null
    private class Pending(val action: () -> Unit) { val identity = Any() }
    private var pending: Pending? = null
    @Volatile var failure: Exception? = null
        private set

    fun submit(action: () -> Unit) {
        val token = synchronized(this) {
            val next = Pending(action)
            pending = next
            if (owner != null) return
            next.identity.also { owner = it }
        }
        launchDrain(token)
    }
    @OptIn(ExperimentalCoroutinesApi::class)
    private fun launchDrain(token: Any) {
        // ATOMIC guarantees finally settles admission even if cancellation wins
        // before the dispatcher starts this job. No native work happens here.
        scope.launch(start = CoroutineStart.ATOMIC) {
            try { drain(token) }
            catch (error: Exception) { if (error !is CancellationException) failed(error) }
            finally { finish(token)?.let(::launchDrain) }
        }
    }
    /** Discard replaceable observations; keep the single scheduled admission. */
    @Synchronized fun clear() { pending = null }

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
        return next?.action
    }
    @Synchronized private fun finish(token: Any): Any? {
        if (owner !== token) return null
        // A cancelled drain may discard its own observation, never a newer one.
        // Transfer admission under the same lock so a concurrent submit cannot
        // be overwritten by a delayed restart. Tokens retain no borrowed view.
        val next = pending?.takeIf { it.identity !== token && scope.isActive }
        pending = next
        owner = next?.identity
        return owner
    }
    private fun failed(error: Exception) {
        failure = error
        try { reportFailure(error) } catch (report: Exception) { if (report !== error) error.addSuppressed(report) }
    }
}
