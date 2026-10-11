package com.blent.connection

import com.blent.CameraCaptureTarget
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers

/** One original private worker's camera publications. Its protocol cannot reuse
 * the camera resource until the previous consumer has actually retired. Keep
 * one unresolved owner and a scalar watermark, never an unbounded route history.
 * The sender captures that worker, independently of the current UI attempt. */
internal class ConnectionCameraSession(
    private val send: (ConnectionWorkerCommand) -> Boolean,
    private val dispatcher: CoroutineDispatcher = Dispatchers.IO,
) {
    private val gate = Any()
    private var accepting = true
    private var ticket: ConnectionTicket? = null
    private var watermark = 0L
    private var owner: ConnectionCameraCaptureOwner? = null

    fun admitted(route: LocalCameraRoute): CameraCaptureTarget? = synchronized(gate) {
        if (!accepting || (ticket != null && ticket != route.lease.connection)) return null
        val previous = owner
        if (previous?.route?.lease == route.lease) {
            return if (sameRoute(previous.route, route)) previous.target else null
        }
        if (previous != null || route.lease.owner <= watermark) return null
        ticket = route.lease.connection
        watermark = route.lease.owner
        ConnectionCameraCaptureOwner(route, send, dispatcher) { delivered(route.lease) }
            .also { owner = it }.target
    }

    private fun sameRoute(first: LocalCameraRoute, second: LocalCameraRoute): Boolean =
        first.port == second.port && first.profile == second.profile &&
            first.credential.bytes().contentEquals(second.credential.bytes())

    private fun delivered(lease: ConnectionResourceLease) {
        synchronized(gate) { if (owner?.route?.lease == lease) owner = null }
    }

    fun closed(lease: ConnectionResourceLease): Boolean {
        val held = synchronized(gate) { owner } ?: return false
        return held.closed(lease)
    }

    /** Visibility/authority loss is not proof of published route closure. */
    fun clear() { synchronized(gate) { owner }?.invalidate() }

    fun stop() {
        val held = synchronized(gate) { accepting = false; owner }
        held?.invalidate()
    }

    /** Invoked only after this session's original connection worker retires. */
    suspend fun retire() {
        val held = synchronized(gate) { accepting = false; owner }
        held?.retire()
        synchronized(gate) { if (owner === held) owner = null }
    }
}
