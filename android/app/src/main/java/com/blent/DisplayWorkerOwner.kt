package com.blent

import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineScope
import java.util.concurrent.atomic.AtomicReference

/** Process-owned display admission. One original native retirement and one latest
 * wanted request are funded by ResourceLane. Clients are opaque, non-reused
 * identities; an obsolete Activity cannot clear a newer client's request.
 * Cleared/replaced requests drop presentation callbacks and unopened factories.
 * The original LifecycleRun remains owned until its exact retirement settles. */
internal class DisplayWorkerOwner(scope: CoroutineScope) {
    @Volatile private var client: Any? = null
    @Volatile private var request: DisplayWorkerRequest? = null
    private val lane = ResourceLane(scope, ::status)

    @Synchronized fun open(): Any {
        val selected = Any()
        client = selected
        discard()
        lane.stop()
        return selected
    }

    fun owns(identity: Any?): Boolean = identity != null && client === identity

    @Synchronized fun clear(identity: Any?) {
        if (!owns(identity)) return
        discard()
        lane.stop()
    }

    @Synchronized fun replace(identity: Any, create: ((WorkerEvent) -> Unit) -> LifecycleRun,
        receive: (WorkerEvent) -> Unit, ended: (Exception?) -> Unit, status: (String) -> Unit) {
        if (!owns(identity)) return
        discard()
        val original = DisplayWorkerRequest(create, receive, ended, status)
        request = original
        try { lane.replace(original::open, original::finished, original::retired) }
        catch (failure: Exception) { discard(); throw failure }
    }

    private fun discard() {
        request?.detach()
        request = null
    }

    // Lane callbacks never acquire the client-operation lock: ResourceLane may
    // report from its own completion/deadline lock on a different thread.
    private fun status(text: String) { request?.status(text) }
}

private class DisplayWorkerRequest(
    create: ((WorkerEvent) -> Unit) -> LifecycleRun,
    receive: (WorkerEvent) -> Unit,
    ended: (Exception?) -> Unit,
    status: (String) -> Unit,
) {
    private val factory = AtomicReference<(((WorkerEvent) -> Unit) -> LifecycleRun)?>(create)
    @Volatile private var receive: ((WorkerEvent) -> Unit)? = receive
    @Volatile private var ended: ((Exception?) -> Unit)? = ended
    @Volatile private var reportStatus: ((String) -> Unit)? = status

    fun open(): LifecycleRun {
        val create = factory.getAndSet(null) ?: throw CancellationException("Display request withdrawn")
        return create(::event)
    }
    private fun event(value: WorkerEvent) { receive?.invoke(value) }
    fun status(text: String) { reportStatus?.invoke(text) }
    fun finished(failure: Exception?) {
        val complete = ended
        ended = null
        receive = null
        complete?.invoke(failure)
    }
    fun retired(proof: ResourceRetirement) {
        factory.set(null)
        if (proof == ResourceRetirement.NOT_STARTED) finished(null)
    }
    fun detach() {
        factory.set(null)
        receive = null
        ended = null
        reportStatus = null
    }
}
