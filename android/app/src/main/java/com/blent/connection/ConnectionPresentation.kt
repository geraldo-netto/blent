package com.blent.connection

import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.asStateFlow

/** Reference identity binds UI actions to one displayed choice, even when a
 * replacement private process starts its ticket counter at the same value. */
internal class ConnectionChoice(val ticket: ConnectionTicket, val tablet: ConnectionPeer)

/** Reference identity also distinguishes equal counters in replacement processes. */
internal class ConnectionDisplayChoice(val route: LocalDisplayRoute)

internal data class ConnectionView(
    val status: String = "USB connection is off.",
    val peer: ConnectionChoice? = null,
    val approval: ConnectionChoice? = null,
    val remembered: Boolean? = null,
    val active: Boolean = false,
    val authenticated: Boolean = false,
    val cleanup: String? = null,
    val display: ConnectionDisplayChoice? = null,
    val cameraOffer: ConnectionCameraOfferChoice? = null,
    val camera: ConnectionCameraRouteChoice? = null,
)

/** Portable presentation reducer. No UI, native work, storage or media consent.
 * A fresh object is the generation token: no wrapping counters or stale aliases. */
internal class ConnectionPresentation {
    private val mutable = MutableStateFlow(ConnectionView())
    val state = mutable.asStateFlow()
    private var generation: Any? = null
    private var accepting = false
    private var finished = true
    private var displayOwner = 0L
    private val camera = ConnectionCameraPresentation()

    @Synchronized fun begin(): Any {
        val token = Any()
        generation = token; accepting = true; finished = false; displayOwner = 0
        camera.reset()
        mutable.value = ConnectionView(status = "Opening USB connection…", active = true,
            cleanup = mutable.value.cleanup)
        return token
    }

    @Synchronized fun event(token: Any, event: ConnectionWorkerEvent) {
        if (generation !== token || finished) return
        when (event) {
            is ConnectionWorkerEvent.Trust -> trust(event)
            is ConnectionWorkerEvent.Ended -> ended(token, event.failure)
            else -> if (accepting) live(event)
        }
    }

    private fun live(event: ConnectionWorkerEvent) {
        when (event) {
            is ConnectionWorkerEvent.Peer -> peer(event.ticket, event.local, event.remembered, false)
            is ConnectionWorkerEvent.Pairing -> peer(event.ticket, event.local, false, true)
            is ConnectionWorkerEvent.State -> phase(event.phase)
            is ConnectionWorkerEvent.Media -> media(event)
            else -> Unit
        }
    }
    private fun media(event: ConnectionWorkerEvent.Media) {
        when (event) {
            is ConnectionWorkerEvent.Display -> display(event.route)
            is ConnectionWorkerEvent.DisplayClosed -> displayClosed(event.lease)
            else -> camera(event)
        }
    }
    private fun camera(event: ConnectionWorkerEvent.Media) {
        val state = mutable.value
        if (!state.authenticated) return
        val ticket = state.peer?.ticket
        when (event) {
            is ConnectionWorkerEvent.CameraOffered -> camera.offered(ticket, event.offer)
            is ConnectionWorkerEvent.Camera -> camera.admitted(ticket, event.route)
            is ConnectionWorkerEvent.CameraClosed -> camera.closed(ticket, event.lease)
            else -> return
        }
        mutable.value = state.copy(cameraOffer = camera.offer, camera = camera.route)
    }

    private fun peer(ticket: ConnectionTicket, local: ConnectionPeer, remembered: Boolean, prompt: Boolean) {
        require(ticket.epoch > 0)
        val choice = ConnectionChoice(ticket, local)
        camera.clear()
        mutable.value = mutable.value.copy(peer = choice, approval = if (prompt) choice else null,
            remembered = remembered, authenticated = false, display = null, cameraOffer = null, camera = null,
            status = if (prompt) "Compare fingerprints on both devices." else "Checking remembered computer…")
    }

    private fun phase(phase: ConnectionWorkerPhase) {
        val text = when (phase) {
            ConnectionWorkerPhase.OPENING -> "Establishing secure USB connection…"
            ConnectionWorkerPhase.WAITING_APPROVAL -> "Waiting for pairing approval…"
            ConnectionWorkerPhase.AUTHENTICATED -> "USB connection authenticated."
            ConnectionWorkerPhase.CLOSING -> "Closing USB connection…"
        }
        val authenticated = phase == ConnectionWorkerPhase.AUTHENTICATED
        val closing = phase == ConnectionWorkerPhase.CLOSING
        if (closing) accepting = false
        cameraPhase(authenticated)
        mutable.value = mutable.value.copy(status = text, authenticated = authenticated,
            approval = if (authenticated || closing) null else mutable.value.approval,
            active = !closing, display = if (authenticated) mutable.value.display else null,
            cameraOffer = camera.offer, camera = camera.route)
    }

    private fun cameraPhase(authenticated: Boolean) { if (!authenticated) camera.clear() }

    private fun display(route: LocalDisplayRoute) {
        val state = mutable.value
        if (!state.authenticated || state.peer?.ticket != route.lease.connection) return
        if (route.lease.owner <= displayOwner) return
        displayOwner = route.lease.owner
        mutable.value = state.copy(display = ConnectionDisplayChoice(route))
    }
    private fun displayClosed(lease: ConnectionResourceLease) {
        val state = mutable.value
        if (state.peer?.ticket != lease.connection) return
        displayOwner = maxOf(displayOwner, lease.owner)
        if (state.display?.route?.lease == lease) mutable.value = state.copy(display = null)
    }

    private fun trust(event: ConnectionWorkerEvent.Trust) {
        if (mutable.value.peer?.ticket?.peer != event.peer) return
        mutable.value = mutable.value.copy(remembered = event.remembered)
    }

    @Synchronized fun decide(token: Any, choice: ConnectionChoice, allow: Boolean, remember: Boolean): ConnectionWorkerCommand? {
        if (generation !== token || !accepting || mutable.value.approval !== choice) return null
        if (!allow) accepting = false
        mutable.value = mutable.value.copy(approval = null, active = allow,
            status = if (allow) "Approval submitted; verifying connection…" else "Computer denied; closing connection…")
        return ConnectionWorkerCommand.Decide(choice.ticket, allow, allow && remember)
    }

    @Synchronized fun revoke(token: Any, choice: ConnectionChoice): ConnectionWorkerCommand? {
        if (generation !== token || !accepting || mutable.value.peer !== choice) return null
        accepting = false
        camera.clear()
        mutable.value = mutable.value.copy(approval = null, active = false, authenticated = false, display = null, cameraOffer = null, camera = null,
            status = "Revoking computer; waiting for saved trust update…")
        return ConnectionWorkerCommand.Revoke(choice.ticket.peer)
    }

    @Synchronized fun stop(token: Any) {
        if (generation !== token || finished) return
        accepting = false
        camera.clear()
        mutable.value = mutable.value.copy(approval = null, active = false, authenticated = false, display = null, cameraOffer = null, camera = null,
            status = "Disconnecting USB…", cleanup = "Closing USB resources…")
    }

    @Synchronized fun ended(token: Any, failure: String?) {
        if (generation !== token || finished) return
        accepting = false
        camera.clear()
        mutable.value = mutable.value.copy(approval = null, active = false, authenticated = false, display = null, cameraOffer = null, camera = null,
            status = failure ?: "USB connection ended.", cleanup = "Closing USB resources…")
    }

    @Synchronized fun started(token: Any) {
        if (generation !== token || finished) return
        mutable.value = mutable.value.copy(cleanup = null)
    }

    @Synchronized fun retired(token: Any) {
        if (generation !== token) return
        finished = true
        camera.clear()
        mutable.value = mutable.value.copy(cleanup = null, display = null, authenticated = false, cameraOffer = null, camera = null)
    }
    @Synchronized fun cameraChoice(token: Any, choice: ConnectionCameraChoice, context: CameraConsentChoice): ConnectionWorkerCommand? {
        if (!cameraCurrent(token, choice)) return null
        return ConnectionWorkerCommand.CameraChoice(choice.lease, context)
    }
    @Synchronized fun cameraPermission(token: Any, choice: ConnectionCameraOfferChoice, granted: Boolean, context: CameraConsentChoice): ConnectionWorkerCommand? {
        if (!cameraCurrent(token, choice) || !choice.offer.permissionRequired) return null
        return ConnectionWorkerCommand.CameraPermission(choice.lease, granted, context)
    }
    private fun cameraCurrent(token: Any, choice: ConnectionCameraChoice): Boolean =
        generation === token && accepting && mutable.value.authenticated && camera.current(choice)

    @Synchronized fun cleanup(text: String) { mutable.value = mutable.value.copy(cleanup = text) }
}
