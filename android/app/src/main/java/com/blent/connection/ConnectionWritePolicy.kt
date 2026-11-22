package com.blent.connection

/** T1760 local next-connection capacity, independent of stream/TLS implementations.
 * Zero borrows original buffers. Positive values fund one reusable payload limit
 * plus 40-byte header; larger frames remain borrowed without a payload copy. */
internal data class ConnectionWritePolicy(val gatherPayloadBytes: Int = 0) {
    init { require(gatherPayloadBytes in 0..16384) }
    fun gathers(payloadBytes: Int): Boolean = payloadBytes in 1..gatherPayloadBytes
}
