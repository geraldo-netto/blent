package com.blent.connection

import com.blent.MediaConsumerAuthority
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers

/** One original worker/resource's publications. One unresolved consumer and a
 * scalar watermark bound storage; no mutable latest-worker sender or history.
 * Metadata projection is pure; native work stays in the retained consumer. */
internal class ConnectionMediaConsumerSession<R : LocalMediaRoute<*>, T>(
    private val commands: ConnectionMediaConsumerCommands,
    private val send: (ConnectionWorkerCommand) -> Boolean,
    private val dispatcher: CoroutineDispatcher = Dispatchers.IO,
    private val project: (R, MediaConsumerAuthority) -> T,
) {
    private class Owned<R, T>(val route: R, val consumer: ConnectionMediaConsumerOwner, val target: T)
    private val gate = Any()
    private var accepting = true
    private var ticket: ConnectionTicket? = null
    private var watermark = 0L
    private var owner: Owned<R, T>? = null
    fun admitted(route: R): T? = synchronized(gate) {
        if (!accepting || route.lease.key.resource != commands.resource || (ticket != null && ticket != route.lease.connection)) return null
        val previous = owner
        if (previous?.route?.lease == route.lease) return if (sameRoute(previous.route, route)) previous.target else null
        if (previous != null || route.lease.owner <= watermark) return null
        ticket = route.lease.connection; watermark = route.lease.owner
        val consumer = ConnectionMediaConsumerOwner(route.lease, commands, send, dispatcher) { delivered(route.lease) }
        Owned(route, consumer, project(route, consumer)).also { owner = it }.target
    }
    private fun sameRoute(first: R, second: R): Boolean = first.port == second.port && first.profile == second.profile &&
        first.credential.bytes().contentEquals(second.credential.bytes())
    private fun delivered(lease: ConnectionResourceLease) {
        synchronized(gate) { if (owner?.route?.lease == lease) owner = null }
    }
    fun closed(lease: ConnectionResourceLease): Boolean {
        val held = synchronized(gate) { owner } ?: return false
        return held.consumer.closed(lease)
    }
    /** Visibility loss is not proof of route closure or consumer/native exit. */
    fun clear() { synchronized(gate) { owner }?.consumer?.invalidate() }
    fun stop() {
        val held = synchronized(gate) { accepting = false; owner }
        held?.consumer?.invalidate()
    }
    /** Only actual original connection-worker retirement reaches this boundary. */
    suspend fun retire() {
        val held = synchronized(gate) { accepting = false; owner }
        held?.consumer?.retire()
        synchronized(gate) { if (owner === held) owner = null }
    }
}
