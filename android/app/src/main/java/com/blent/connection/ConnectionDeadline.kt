package com.blent.connection

/** Immutable elapsed-realtime budget snapshot, never native retirement proof.
 * known means every contributing owner exposes its budgets. Unknown providers
 * still require the legacy polling fallback; other known deadlines remain useful.
 * A null deadline charges no idle time and grants no resource admission/reuse. */
internal data class ConnectionDeadline(val atMs: Long?, val known: Boolean) {
    init { require(atMs == null || atMs >= 0) }

    fun combine(other: ConnectionDeadline): ConnectionDeadline {
        val first = atMs
        val second = other.atMs
        val earliest = when {
            first == null -> second
            second == null -> first
            else -> minOf(first, second)
        }
        val complete = known && other.known
        if (earliest == first && complete == known) return this
        if (earliest == second && complete == other.known) return other
        return ConnectionDeadline(earliest, complete)
    }

    companion object {
        val IDLE = ConnectionDeadline(null, true)
        val UNKNOWN = ConnectionDeadline(null, false)
        fun at(time: Long?): ConnectionDeadline = if (time == null) IDLE else ConnectionDeadline(time, true)
        fun observe(source: ConnectionDeadlineSource?): ConnectionDeadline = source?.nextDeadline() ?: UNKNOWN
    }
}

/** Optional portable capability. Missing capability is UNKNOWN, never idle.
 * Read on the same serialized owner as its transitions; returns no borrowed view. */
internal interface ConnectionDeadlineSource {
    fun nextDeadline(): ConnectionDeadline
}
