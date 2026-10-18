package com.blent

/** Withdrawal never depends on a vanished source reader. Invalid/failing desired
 * views remain unadmitted until a fresh current view is actually delivered. */
internal fun DisplayPowerRequest.permitsPower(): Boolean {
    if (power == null) return true
    return try { permit.current() } catch (_: Exception) { false }
}
