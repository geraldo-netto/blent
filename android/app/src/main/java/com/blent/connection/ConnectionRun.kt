package com.blent.connection

/** Resource contract shared by the serialized protocol owner and native runner.
 * Endpoints are allocated locally; a remote key never selects an OS address. */
internal interface ConnectionEndpoints {
    fun nextEvent(): ConnectionSessionEvent?
    fun prepared(lease: ConnectionResourceLease, now: Long)
    fun permissionRequired(lease: ConnectionResourceLease, now: Long)
    fun reject(lease: ConnectionResourceLease, outcome: ConnectionManagement.Outcome)
    fun cancel(lease: ConnectionResourceLease, reason: ConnectionManagement.Reason)
    fun current(lease: ConnectionResourceLease): Boolean
    /** True transfers this array; the caller must never mutate/reuse it. */
    fun offer(lease: ConnectionResourceLease, channel: Int, bytes: ByteArray): Boolean
    fun read(lease: ConnectionResourceLease, channel: Int, maxBytes: Int): ConnectionChannelRead?
    fun consumed(lease: ConnectionResourceLease, channel: Int, receipt: ConnectionChannelRead)
    /** Actual local endpoint exit, never a cancellation/timeout notification. */
    fun nativeRetired(lease: ConnectionResourceLease)
}

internal data class ConnectionRunLimits(val ioTimeoutMs: Long = 5000, val retirementTimeoutMs: Long = 5000) {
    init { require(ioTimeoutMs in 1..60000 && retirementTimeoutMs in 1..60000) }
}

/** Single serialized owner; advance must run on an independent supervisor clock
 * even when native I/O blocks. Endpoint operations are serialized with advance.
 * This contract owns transport I/O, not local endpoint workers or process exit. */
internal interface ConnectionRun {
    /** Current locally admitted display; null revokes visibility, not ownership. */
    fun display(): LocalDisplayRoute?
    fun endpoints(): ConnectionEndpoints
    fun advance(now: Long)
    fun stop(now: Long)
    fun failure(): Exception?
    /** Native read/write/close calls exited; OS process admission remains separate. */
    fun retired(): Boolean
    /** Requires supervised process exit; never grants replacement admission. */
    fun terminationRequired(): Boolean
}
