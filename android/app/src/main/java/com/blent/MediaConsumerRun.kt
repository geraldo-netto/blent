package com.blent

import kotlinx.coroutines.currentCoroutineContext
import kotlinx.coroutines.ensureActive
import java.util.concurrent.atomic.AtomicBoolean

/** Lazy native acquisition: permission and the exact retained claim are checked
 * before factory/native execution. All created workers remain owned by retire. */
internal class MediaConsumerRun(
    private val claim: MediaConsumerClaim?,
    private val permission: () -> Boolean,
    private val create: () -> LifecycleRun,
) : LifecycleRun {
    private val stopped = AtomicBoolean()
    private val admitted = AtomicBoolean()
    @Volatile private var worker: LifecycleRun? = null
    override suspend fun execute() {
        currentCoroutineContext().ensureActive()
        check(admitted.compareAndSet(false, true)) { "Media request already executed" }
        check(!stopped.get()) { "Media request retired" }
        check(permission()) { "Media permission unavailable" }
        check(claim?.current() != false) { "Media target no longer current" }
        val run = create()
        worker = run
        currentCoroutineContext().ensureActive()
        check(!stopped.get()) { "Media request retired" }
        check(permission()) { "Media permission unavailable" }
        check(claim?.current() != false) { "Media target no longer current" }
        run.execute()
    }
    override fun cancel() { stopped.set(true); worker?.cancel() }
    override suspend fun retire() { cancel(); worker?.retire() }
}
