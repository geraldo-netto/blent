package com.blent.connection

/** Immutable scheduling view; no device, queue capacity or resource authority.
 * budgetMs bounds the shortest asynchronously introduced configured budget. */
internal data class ConnectionWaitLimits(val fallbackMs: Long, val budgetMs: Long) {
    init {
        require(fallbackMs in 1..1000)
        require(budgetMs in 1..600000)
    }
}

/** Pure relative scheduling from independent absolute budget metadata.
 * A due result requests another owner advance, never native completion. */
internal object ConnectionWait {
    fun fallback(limits: ConnectionWaitLimits, retiring: Boolean): Long {
        val requested = if (retiring) minOf(limits.fallbackMs, 10) else limits.fallbackMs
        return minOf(requested, limits.budgetMs)
    }
    fun delayMs(now: Long, deadline: ConnectionDeadline, fallbackMs: Long): Long {
        require(now >= 0)
        require(fallbackMs in 1..1000)
        val interval = if (deadline.known) fallbackMs else minOf(fallbackMs, 10)
        val due = deadline.atMs ?: return interval
        if (due <= now) return 0
        return minOf(interval, due - now)
    }
}
