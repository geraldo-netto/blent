package com.blent

import com.blent.activity.DisplayActivityUpdate
import kotlinx.coroutines.*
import kotlin.coroutines.CoroutineContext
import java.util.concurrent.atomic.AtomicBoolean

/** One scheduled UI drain and one latest observation across foreground epochs.
 * Stop clears borrowed views; old callbacks cannot join a replacement epoch. */
internal class DisplayActivityDelivery(
    dispatchUi: (() -> Unit) -> Unit,
    private val consume: (DisplayActivityUpdate) -> Unit,
    reportFailure: (Exception) -> Unit,
) {
    private class UiDispatch(
        private val dispatchUi: (() -> Unit) -> Unit,
        private val reportFailure: (Exception) -> Unit,
    ) : CoroutineDispatcher() {
        override fun dispatch(context: CoroutineContext, block: Runnable) {
            val once = AtomicBoolean()
            // Always schedule first: an inline UI dispatcher cannot recurse on
            // yield. The guard also fences callbacks queued before rejection.
            Dispatchers.Default.dispatch(context) {
                try { dispatchUi { if (once.compareAndSet(false, true)) block.run() } }
                catch (failure: Exception) { reject(context, block, once, failure) }
            }
        }
        private fun reject(context: CoroutineContext, block: Runnable, once: AtomicBoolean, failure: Exception) {
            context[Job]?.cancel(CancellationException("Display UI dispatch rejected", failure))
            // Cancellation makes the original drain run only its cleanup. A
            // queued late callback loses this exact continuation guard.
            if (once.compareAndSet(false, true)) block.run()
            try { reportFailure(failure) }
            catch (report: Exception) { if (report !== failure) failure.addSuppressed(report) }
        }
    }
    private val scope = CoroutineScope(SupervisorJob() + UiDispatch(dispatchUi, reportFailure))
    private val latest = LatestDispatch(scope, reportFailure)
    @Volatile private var owner: Any? = null
    fun start() { latest.clear(); owner = Any() }
    fun submit(update: DisplayActivityUpdate) {
        val original = owner ?: return
        latest.submit { if (owner === original) consume(update) }
    }
    fun stop() { owner = null; latest.clear() }
}
