package com.blent

/** Byte admission contract, independent of socket and UI implementations. */
internal object ControlCapacity {
    const val DEFAULT = 65_536
    const val MIN = 8_192
    const val MAX = 1_048_576

    fun allows(queued: Long, incoming: Long, budget: Int): Boolean =
        budget in MIN..MAX && queued >= 0 && incoming > 0 && queued <= budget && incoming <= budget - queued
}
