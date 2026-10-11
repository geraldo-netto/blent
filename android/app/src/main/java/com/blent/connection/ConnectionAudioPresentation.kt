package com.blent.connection

internal sealed interface ConnectionAudioChoice : ConnectionMediaChoice
internal class ConnectionAudioOfferChoice(val offer: LocalAudioOffer) : ConnectionAudioChoice {
    override val lease: ConnectionResourceLease get() = offer.lease
}
internal class ConnectionAudioRouteChoice(val route: LocalAudioRoute) : ConnectionAudioChoice {
    override val lease: ConnectionResourceLease get() = route.lease
}
/** One independent direction's choice metadata, outside native audio APIs. */
internal class ConnectionAudioPresentation(resource: ConnectionManagement.Resource) {
    init { require(resource == ConnectionManagement.Resource.MICROPHONE || resource == ConnectionManagement.Resource.SPEAKERS) }
    private val projection = ConnectionMediaPresentation<LocalAudioOffer, LocalAudioRoute, ConnectionAudioOfferChoice, ConnectionAudioRouteChoice>(
        resource, ::ConnectionAudioOfferChoice, ::ConnectionAudioRouteChoice,
    )
    val offer: ConnectionAudioOfferChoice? get() = projection.offer
    val route: ConnectionAudioRouteChoice? get() = projection.route
    fun reset() = projection.reset()
    fun clear() = projection.clear()
    fun offered(ticket: ConnectionTicket?, value: LocalAudioOffer) = projection.offered(ticket, value)
    fun admitted(ticket: ConnectionTicket?, value: LocalAudioRoute) = projection.admitted(ticket, value)
    fun closed(ticket: ConnectionTicket?, lease: ConnectionResourceLease) = projection.closed(ticket, lease)
    fun current(choice: ConnectionAudioChoice): Boolean = projection.current(choice)
}
