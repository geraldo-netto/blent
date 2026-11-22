package com.blent

/** T1726: TLS reads consume the original approval budget, without resetting it.
 * Pure value adapter: no socket, worker or retained callback ownership. */
internal fun aoaSetupRemaining(deadline: Long, now: Long, stopped: Boolean): Int {
    require(deadline >= 0 && now >= 0)
    check(!stopped) { "Approval cancelled" }
    val remaining = deadline - now
    check(remaining in 1..120000) { "Approval expired during TLS setup" }
    return remaining.toInt()
}
