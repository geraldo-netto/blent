package com.blent.connection

import com.blent.LifecycleRun
import com.blent.ResourceLane
import kotlinx.coroutines.CoroutineScope

/** Native-inert construction; ordered callbacks settle before execute returns.
 * Successful retire proves original private-process/store/IPC quiescence.
 * Rejection and failed acquisition remain owned by that original run. */
internal fun interface SavedTrustRunFactory {
    fun create(request: SavedTrustRequest, report: (SavedComputersPage) -> Unit): LifecycleRun
}

/** One current UI client, one accepted transaction and one bounded result.
 * Application scope and the original lane retain native work after UI loss.
 * Public actions serialize separately from reducer-only callbacks. */
internal class SavedTrustBinding(
    scope: CoroutineScope,
    private val access: ConnectionAccess,
    private val factory: SavedTrustRunFactory,
    cleanupNoticeMs: Long = 5000,
) {
    private class Attempt(val token: Any) {
        val collector = SavedTrustCollector()
        var result: SavedTrustResult? = null
    }
    private val presentation = SavedTrustPresentation()
    val state = presentation.state
    val connectionUse = access.changes
    private val lane = ResourceLane(scope, ::cleanup, cleanupNoticeMs)
    @Volatile private var current: Attempt? = null

    @Synchronized fun attach(): SavedTrustClient = SavedTrustClient(this, presentation.attach())
    @Synchronized internal fun detach(viewer: Any) = presentation.detach(viewer)
    @Synchronized internal fun refresh(viewer: Any): Boolean = start(viewer, SavedTrustRequest.List)
    @Synchronized internal fun choice(viewer: Any, peer: ConnectionPeer): SavedTrustChoice? = presentation.choice(viewer, peer)
    @Synchronized internal fun forget(viewer: Any, choice: SavedTrustChoice): Boolean {
        val selected = presentation.selection(viewer, choice) ?: return false
        return start(viewer, SavedTrustRequest.Forget(selected))
    }
    @Synchronized internal fun stop(viewer: Any): Boolean {
        if (!presentation.stop(viewer)) return false
        lane.stop()
        return true
    }
    private fun start(viewer: Any, request: SavedTrustRequest): Boolean {
        if (access.current() != null || !presentation.canRequest(viewer)) return false
        lane.checkAvailable()
        val token = presentation.begin(viewer) ?: return false
        val attempt = Attempt(token)
        current = attempt
        lane.replace({ open(attempt, request) },
            { presentation.ended(token, it) }, { presentation.retired(token, attempt.result) })
        return true
    }
    private fun open(attempt: Attempt, request: SavedTrustRequest): LifecycleRun {
        val raw = factory.create(request) { page ->
            presentation.consume(attempt.token) { attempt.collector.add(page) }
        }
        val complete = object : LifecycleRun {
            override suspend fun execute() {
                raw.execute()
                attempt.result = attempt.collector.result()
            }
            override fun cancel() = raw.cancel()
            override suspend fun retire() = raw.retire()
        }
        return access.guard(ConnectionUse.SAVED_TRUST, complete)
    }
    private fun cleanup(text: String) {
        val attempt = current ?: return
        presentation.cleanup(attempt.token, text, lane.error != null, lane.stalled)
    }
}

/** View-bound command ticket. Closing revokes commands, never accepted work. */
internal class SavedTrustClient internal constructor(private val binding: SavedTrustBinding, private val viewer: Any) : AutoCloseable {
    val current: Boolean get() = binding.state.value.viewer === viewer
    fun refresh(): Boolean = binding.refresh(viewer)
    fun choice(peer: ConnectionPeer): SavedTrustChoice? = binding.choice(viewer, peer)
    fun forget(choice: SavedTrustChoice): Boolean = binding.forget(viewer, choice)
    fun stop(): Boolean = binding.stop(viewer)
    override fun close() = binding.detach(viewer)
}
