package com.blent.connection

/** Serialized change-only private projection. Logical closure revokes visibility,
 * never actual native ownership. Offer-to-admission keeps the same exact lease. */
internal class ConnectionCameraPublication(private val report: (ConnectionWorkerEvent) -> Unit) {
    private var offered: LocalCameraOffer? = null
    private var route: LocalCameraRoute? = null
    fun update(offer: LocalCameraOffer?, selected: LocalCameraRoute?) {
        require(offer == null || selected == null)
        if (offered == offer && route == selected) return
        val previous = route?.lease ?: offered?.lease
        val next = selected?.lease ?: offer?.lease
        offered = offer; route = selected // Retain transition before callbacks.
        if (previous != null && previous != next) report(ConnectionWorkerEvent.CameraClosed(previous))
        when {
            selected != null -> report(ConnectionWorkerEvent.Camera(selected))
            offer != null -> report(ConnectionWorkerEvent.CameraOffered(offer))
        }
    }
}
