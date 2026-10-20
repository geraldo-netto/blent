package com.blent.connection

import com.blent.LifecycleRun
import kotlinx.coroutines.CompletableDeferred

/** Metadata-only UI fixture. Actual private Binder ownership is covered by
 * AndroidSavedTrustRunTest and the retained native retirement suite. */
internal class SavedTrustUiRun(val report: (SavedComputersPage) -> Unit) : LifecycleRun {
    val finish = CompletableDeferred<Unit>()
    val release = CompletableDeferred<Unit>()
    var executed = false
    var cancelled = 0
    var failure: Exception? = null
    var closeFailure: Exception? = null
    override suspend fun execute() { executed = true; finish.await(); failure?.let { throw it } }
    override fun cancel() { cancelled++ }
    override suspend fun retire() {
        if (!executed) return
        release.await(); closeFailure?.let { throw it }
    }
    fun complete(total: Int = 1, retire: Boolean = true) {
        savedPages(total).forEach(report)
        if (retire) release.complete(Unit)
        finish.complete(Unit)
    }
}
