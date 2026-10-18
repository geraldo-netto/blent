package com.blent

/** Withdrawal never depends on a vanished source reader. Invalid/failing desired
 * views remain unadmitted until a fresh current view is actually delivered. */
internal fun DisplayPowerRequest.permitsPower(): Boolean {
    if (power == null) return true
    return permit.permitsPower()
}

/** Shared pure reader-fault boundary for independent service/window consumers. */
internal fun DisplayPowerPermit.permitsPower(): Boolean = try { current() } catch (_: Exception) { false }
