package com.blent.connection

/** Owned native lifetime. close may block; retired reports actual completion,
 * never merely a stop request. retired must return promptly without native I/O.
 * Implementations must make close idempotent. */
internal interface ConnectionSetupLifetime {
    fun close()
    fun retired(): Boolean
}

/** Blocking boundary, invoked only by setup's native worker. close may run
 * concurrently on the separate closer; all later-created handles remain owned.
 * Peer identity returned by authenticate is stock TLS proof, never HELLO input. */
internal interface ConnectionSetupBackend : ConnectionSetupLifetime {
    fun localIdentity(): ConnectionIdentity
    fun discover(timeoutMs: Long): ConnectionIdentity
    fun ready(peer: ConnectionIdentity, readinessTimeoutMs: Long)
    fun authenticate(peer: ConnectionIdentity): ConnectionPeer
    fun negotiate(capacity: Int, timeoutMs: Long): Int
    fun io(wake: () -> Unit): ConnectionIo
}

internal data class ConnectionSetupLimits(
    val capacity: Int = 262144,
    val approvalTimeoutMs: Long = 120000,
    val nativeTimeoutMs: Long = 10000,
    val retirementTimeoutMs: Long = 2000,
    val trustedPeers: Int = 64,
) {
    init {
        ConnectionWindow.validate(capacity)
        require(trustedPeers in 1..1024)
        require(approvalTimeoutMs in 1..600000)
        require(nativeTimeoutMs in 1..60000 && retirementTimeoutMs in 1..60000)
    }
}

/** Borrowed runtime handles: setup retains native ownership through stop and
 * actual retirement. The session must use setup.current as its authority gate. */
internal class ConnectionSetupReady internal constructor(
    val io: ConnectionIo,
    val approval: ConnectionApproval,
    val capacity: Int,
)

/** Serialized control-loop API. Methods are prompt; native work/storage never
 * runs here. Wake callbacks are concurrent hints, requiring serialized dispatch.
 * advance must also run from an independent timer while native calls are stuck. */
internal interface ConnectionSetup {
    fun advance()
    fun decide(ticket: ConnectionTicket, approved: Boolean, remember: Boolean): Boolean
    fun revoke(peer: ConnectionPeer): Boolean
    fun current(approval: ConnectionApproval): Boolean
    fun takeReady(): ConnectionSetupReady?
    fun poll(): ConnectionWorkerEvent?
    fun stop()
    fun retired(): Boolean
    fun failure(): Exception?
    fun terminationRequired(): Boolean
}
