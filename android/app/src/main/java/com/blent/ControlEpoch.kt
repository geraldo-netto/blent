package com.blent

/** Delivery identity only. A notice cannot grant native admission or retirement. */
internal class ControlNotice

/** Checked admission and serialized delivery identity share one controller owner.
 * Revocation and denied admission still publish fresh status/cleanup notices. */
internal class ControlEpoch(initial: Long = 0) {
    private val grants = GenerationFence(initial)
    private var notice = ControlNotice()

    @Synchronized fun acquire(): Boolean {
        notice = ControlNotice()
        return grants.acquire() != null
    }

    @Synchronized fun revoke() {
        notice = ControlNotice()
        grants.revoke()
    }

    @Synchronized fun available(): Boolean = grants.available()
    @Synchronized fun snapshot(): ControlNotice = notice
    @Synchronized fun current(original: ControlNotice): Boolean = notice === original
}
