package com.blent

/** Instance-local callback authority. Revocation needs no new number and cannot
 * retire native resources. Each replacement invalidates the previous grant,
 * including a denied replacement at the terminal positive-Long boundary. */
internal class GenerationFence(initial: Long = 0) {
    private var revision = initial
    private var active = false
    init { require(initial >= 0) }

    @Synchronized fun acquire(): Long? {
        active = false
        if (revision == Long.MAX_VALUE) return null
        revision += 1
        active = true
        return revision
    }

    @Synchronized fun revoke() { active = false }

    @Synchronized fun current(value: Long): Boolean = active && value == revision
}
