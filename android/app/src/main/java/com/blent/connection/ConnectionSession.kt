package com.blent.connection

/** Portable runtime contracts. Implementations own protocol policy; native
 * adapters own threads, sockets, permissions and actual retirement evidence.
 * Frame/consumer buffers stay inside the connection process, never on Binder. */
internal data class ConnectionSessionLimits(
    val capacity: Int = 262144, val messages: Int = 64,
    val quantum: Int = 65536, val resourceTimeoutMs: Long = 120000,
) {
    init {
        ConnectionWindow.validate(capacity)
        require(messages in 1..1024 && quantum in 1..ConnectionWire.MAX_PAYLOAD)
        require(resourceTimeoutMs in 1..600000)
    }
}

internal enum class ConnectionResourceState { ADMITTED, CLOSING, RETIRED }
internal sealed interface ConnectionSessionEvent {
    data class Offered(val lease: ConnectionResourceLease, val credential: ConnectionManagement.Credential,
        val profile: ConnectionManagement.Profile) : ConnectionSessionEvent
    data class State(val lease: ConnectionResourceLease, val state: ConnectionResourceState) : ConnectionSessionEvent
    data class Readable(val lease: ConnectionResourceLease, val channel: Int) : ConnectionSessionEvent
}

internal interface ConnectionSession {
    /** Consume a complete validated frame synchronously; no input loan retained. */
    fun receive(frame: ConnectionFrame, now: Long)
    fun nextFrame(now: Long): ConnectionFrame?
    fun written(frame: ConnectionFrame, now: Long)
    fun nextEvent(): ConnectionSessionEvent?
    fun prepared(lease: ConnectionResourceLease, now: Long)
    fun permissionRequired(lease: ConnectionResourceLease, now: Long)
    fun reject(lease: ConnectionResourceLease, outcome: ConnectionManagement.Outcome)
    fun cancel(lease: ConnectionResourceLease, reason: ConnectionManagement.Reason)
    fun current(lease: ConnectionResourceLease): Boolean
    /** True transfers the array to this owner. Caller must never mutate/reuse it.
     * False retains caller ownership; bounded capacity supplies backpressure. */
    fun offer(lease: ConnectionResourceLease, channel: Int, bytes: ByteArray): Boolean
    /** Borrow one contiguous slice until consumed or actual native retirement. */
    fun read(lease: ConnectionResourceLease, channel: Int, maxBytes: Int): ConnectionChannelRead?
    fun consumed(lease: ConnectionResourceLease, channel: Int, receipt: ConnectionChannelRead)
    fun nativeRetired(lease: ConnectionResourceLease)
    fun tick(now: Long)
    fun disconnect()
    fun transportRetired()
}
