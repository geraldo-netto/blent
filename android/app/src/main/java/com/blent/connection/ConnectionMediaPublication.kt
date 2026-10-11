package com.blent.connection

/** Change-only media publication: retain transition before callbacks; close an
 * old lease before exposing a replacement. Metadata is never native proof. */
internal class ConnectionMediaPublication<O : LocalMediaOffer<*>, R : LocalMediaRoute<*>>(
    private val close: (ConnectionResourceLease) -> Unit, private val offer: (O) -> Unit, private val admit: (R) -> Unit,
) {
    private var offered: O? = null
    private var route: R? = null
    fun update(offer: O?, selected: R?) {
        require(offer == null || selected == null)
        if (offered == offer && route == selected) return
        val previous = route?.lease ?: offered?.lease
        val next = selected?.lease ?: offer?.lease
        offered = offer; route = selected // Retain transition before callbacks.
        if (previous != null && previous != next) close(previous)
        when {
            selected != null -> admit(selected)
            offer != null -> this.offer(offer)
        }
    }
}
