package com.blent.connection

/** SHA-256 of exact certificate DER, computed by the stock TLS boundary. */
internal data class ConnectionPeer private constructor(val fingerprint: String) {
    init { require(fingerprint.length == 64 && fingerprint.all { it in '0'..'9' || it in 'a'..'f' }) }
    companion object { fun parse(value: String): ConnectionPeer = ConnectionPeer(value) }
}

/** Independent authority values; constructing one grants no proof or permission. */
internal data class ConnectionTicket(val epoch: Long, val peer: ConnectionPeer)
internal data class ConnectionApproval(val ticket: ConnectionTicket, val remember: Boolean)
internal data class ConnectionResourceLease(
    val connection: ConnectionTicket, val key: ConnectionManagement.Key, val owner: Long,
)
