package com.blent.connection

import java.util.concurrent.atomic.AtomicBoolean
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.withTimeout
import kotlinx.coroutines.currentCoroutineContext
import kotlinx.coroutines.ensureActive

/** One-shot authority job. execute/close run only on the private service's
 * retained IO job. Cancellation is a cutoff; it cannot interrupt native calls.
 * Failed creation/close requires the enclosing original process-death fence. */
internal class SavedTrustWorker(
    private val request: SavedTrustRequest,
    private val open: () -> SavedTrustAccess,
    private val report: (ConnectionWorkerEvent) -> Unit,
) : ConnectionWorker {
    private val started = AtomicBoolean()
    private val stopped = AtomicBoolean()
    private val completed = CompletableDeferred<Unit>()
    @Volatile private var uncertain = false

    override suspend fun execute() {
        check(started.compareAndSet(false, true)) { "Saved-trust worker already started or retired" }
        var access: SavedTrustAccess? = null
        try {
            if (stopped.get()) return
            currentCoroutineContext().ensureActive()
            uncertain = true
            val created = open()
            access = created
            uncertain = false
            if (stopped.get()) return
            currentCoroutineContext().ensureActive()
            if (request is SavedTrustRequest.Forget) created.forget(request.selected)
            currentCoroutineContext().ensureActive()
            if (!stopped.get()) publish(created.snapshot())
        } finally { finish(access) }
    }
    private fun finish(access: SavedTrustAccess?) {
        try { access?.close(); completed.complete(Unit) }
        catch (failure: Exception) { completed.completeExceptionally(failure); throw failure }
    }
    private suspend fun publish(snapshot: SavedComputers?) {
        val peers = snapshot?.peers ?: emptyList()
        if (peers.isEmpty()) {
            page(snapshot, 0, 0, emptyList())
            return
        }
        for (offset in peers.indices step SavedComputersPage.CAPACITY) {
            if (stopped.get()) return
            val end = minOf(peers.size, offset + SavedComputersPage.CAPACITY)
            page(snapshot, peers.size, offset, peers.subList(offset, end))
        }
    }
    private suspend fun page(snapshot: SavedComputers?, total: Int, offset: Int, peers: List<ConnectionPeer>) {
        if (stopped.get()) return
        currentCoroutineContext().ensureActive()
        report(ConnectionWorkerEvent.SavedComputers(SavedComputersPage(
            snapshot?.local, snapshot?.revision ?: 0, total, offset, peers,
        )))
    }
    override fun command(value: ConnectionWorkerCommand): Boolean = false
    override fun cancel() { stopped.set(true) }
    override suspend fun retire() {
        cancel()
        if (started.compareAndSet(false, true)) completed.complete(Unit)
        withTimeout(5000) { completed.await() }
        check(!uncertain) { "Saved-trust factory ownership is unconfirmed; original process exit required" }
    }
}
