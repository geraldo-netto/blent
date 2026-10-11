package com.blent.connection

/** Choices bind UI callbacks to this exact original publication, not counters. */
internal sealed interface ConnectionCameraChoice { val lease: ConnectionResourceLease }
internal class ConnectionCameraOfferChoice(val offer: LocalCameraOffer) : ConnectionCameraChoice {
    override val lease: ConnectionResourceLease get() = offer.lease
}
internal class ConnectionCameraRouteChoice(val route: LocalCameraRoute) : ConnectionCameraChoice {
    override val lease: ConnectionResourceLease get() = route.lease
}

/** Portable camera-only projection. Original worker generation/authentication
 * is checked by the outer reducer; values themselves grant no native permission. */
internal class ConnectionCameraPresentation {
    private var owner = 0L
    var offer: ConnectionCameraOfferChoice? = null; private set
    var route: ConnectionCameraRouteChoice? = null; private set
    fun reset() { owner = 0; clear() }
    fun clear() { offer = null; route = null }
    fun offered(ticket: ConnectionTicket?, value: LocalCameraOffer) {
        if (value.lease.connection != ticket || value.lease.owner < owner) return
        if (value.lease.owner == owner && offer?.lease != value.lease) return
        val current = offer
        if (current != null && value.lease.owner == owner && current.offer.profile != value.profile) return
        if (current?.offer == value) return
        owner = value.lease.owner
        offer = ConnectionCameraOfferChoice(value); route = null
    }
    fun admitted(ticket: ConnectionTicket?, value: LocalCameraRoute) {
        if (value.lease.connection != ticket || value.lease.owner < owner) return
        if (value.lease.owner == owner && offer?.lease != value.lease) return
        if (offer != null && value.lease.owner == owner && offer?.offer?.profile != value.profile) return
        owner = value.lease.owner
        route = ConnectionCameraRouteChoice(value); offer = null
    }
    fun closed(ticket: ConnectionTicket?, lease: ConnectionResourceLease) {
        if (lease.connection != ticket) return
        owner = maxOf(owner, lease.owner)
        if (offer?.lease == lease || route?.lease == lease) clear()
    }
    fun current(choice: ConnectionCameraChoice): Boolean = offer === choice || route === choice
}
