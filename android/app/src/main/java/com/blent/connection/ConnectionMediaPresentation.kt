package com.blent.connection

/** Shared portable offer/admission projection with one scalar owner watermark.
 * Outer reducer retains exact worker authentication; this owner grants no native
 * permission. Metadata and choice constructors are explicit boundary adapters. */
internal class ConnectionMediaPresentation<O : LocalMediaOffer<*>, R : LocalMediaRoute<*>, OC : ConnectionMediaChoice, RC : ConnectionMediaChoice>(
    private val resource: ConnectionManagement.Resource,
    private val offerChoice: (O) -> OC,
    private val routeChoice: (R) -> RC,
) {
    private var owner = 0L
    private var metadata: O? = null
    var offer: OC? = null; private set
    var route: RC? = null; private set
    fun reset() { owner = 0; clear() }
    fun clear() { metadata = null; offer = null; route = null }
    private fun allowed(ticket: ConnectionTicket?, lease: ConnectionResourceLease): Boolean =
        lease.key.resource == resource && lease.connection == ticket && lease.owner >= owner
    fun offered(ticket: ConnectionTicket?, value: O) {
        if (!allowed(ticket, value.lease)) return
        if (value.lease.owner == owner && offer?.lease != value.lease) return
        val current = metadata
        if (current != null && value.lease.owner == owner && current.profile != value.profile) return
        if (current == value) return
        owner = value.lease.owner; metadata = value
        offer = offerChoice(value); route = null
    }
    fun admitted(ticket: ConnectionTicket?, value: R) {
        if (!allowed(ticket, value.lease)) return
        if (value.lease.owner == owner && offer?.lease != value.lease) return
        if (offer != null && value.lease.owner == owner && metadata?.profile != value.profile) return
        owner = value.lease.owner
        route = routeChoice(value); offer = null; metadata = null
    }
    fun closed(ticket: ConnectionTicket?, lease: ConnectionResourceLease) {
        if (lease.key.resource != resource || lease.connection != ticket) return
        owner = maxOf(owner, lease.owner)
        if (offer?.lease == lease || route?.lease == lease) clear()
    }
    fun current(choice: ConnectionMediaChoice): Boolean = offer === choice || route === choice
}
