package com.blent.connection

/** Choices bind UI callbacks to this exact original publication, not counters. */
internal sealed interface ConnectionCameraChoice : ConnectionMediaChoice
internal class ConnectionCameraOfferChoice(val offer: LocalCameraOffer) : ConnectionCameraChoice {
    override val lease: ConnectionResourceLease get() = offer.lease
}
internal class ConnectionCameraRouteChoice(val route: LocalCameraRoute) : ConnectionCameraChoice {
    override val lease: ConnectionResourceLease get() = route.lease
}

/** Camera choice adapter; common resource projection owns admission/watermarks. */
internal class ConnectionCameraPresentation {
    private val projection = ConnectionMediaPresentation<LocalCameraOffer, LocalCameraRoute, ConnectionCameraOfferChoice, ConnectionCameraRouteChoice>(
        ConnectionManagement.Resource.CAMERA, ::ConnectionCameraOfferChoice, ::ConnectionCameraRouteChoice,
    )
    val offer: ConnectionCameraOfferChoice? get() = projection.offer
    val route: ConnectionCameraRouteChoice? get() = projection.route
    fun reset() = projection.reset()
    fun clear() = projection.clear()
    fun offered(ticket: ConnectionTicket?, value: LocalCameraOffer) = projection.offered(ticket, value)
    fun admitted(ticket: ConnectionTicket?, value: LocalCameraRoute) = projection.admitted(ticket, value)
    fun closed(ticket: ConnectionTicket?, lease: ConnectionResourceLease) = projection.closed(ticket, lease)
    fun current(choice: ConnectionCameraChoice): Boolean = projection.current(choice)
}
