package com.blent.connection

import java.net.Socket

/** Lazily composes USB with stock TLS on the setup native thread. Construction
 * does not query Android services or storage. Retain this owner on every error. */
internal class AndroidConnectionSetupBackend(
    private val opening: () -> AndroidAccessoryOpening,
    private val identity: () -> ClientTlsIdentity,
    private val socketFactory: (Socket, () -> ClientTlsIdentity, ConnectionSetupLifetime) -> ConnectionSetupBackend =
        { socket, local, lifetime -> SocketConnectionSetup(socket, local, lifetime) },
) : ConnectionSetupBackend {
    private val gate = Any()
    private var started = false
    private var initializing = false
    private var closing = false
    private var closed = false
    private var uncertain = false
    private var failedLateClose = false
    private var accessory: AndroidAccessoryOpening? = null
    private var delegate: ConnectionSetupBackend? = null

    private fun live() { synchronized(gate) { check(!closing) { "USB setup is closing" } } }
    private fun adoptOpening(value: AndroidAccessoryOpening) {
        val late = synchronized(gate) { accessory = value; uncertain = false; closing }
        if (late) lateClose(value)
    }
    private fun adoptSocket(value: ConnectionSetupBackend) {
        val late = synchronized(gate) { delegate = value; uncertain = false; closing }
        if (late) lateClose(value)
    }
    private fun lateClose(value: ConnectionSetupLifetime): Nothing {
        try { value.close() }
        catch (failure: Exception) { synchronized(gate) { failedLateClose = true }; throw failure }
        error("Native setup completed after cancellation")
    }
    private fun initialize(): ConnectionSetupBackend {
        synchronized(gate) {
            check(!started && !closing) { "USB setup already attempted or cancelled" }
            started = true; initializing = true; uncertain = true
        }
        try {
            val owned = opening(); adoptOpening(owned)
            val socket = owned.open()
            live()
            synchronized(gate) { uncertain = true }
            val created = socketFactory(socket, identity, owned)
            adoptSocket(created)
            return created
        } finally { synchronized(gate) { initializing = false } }
    }
    private fun current(): ConnectionSetupBackend {
        live()
        return synchronized(gate) { checkNotNull(delegate) { "USB setup has not opened" } }
    }
    override fun localIdentity(): ConnectionIdentity {
        live()
        val current = synchronized(gate) { delegate } ?: initialize()
        return current.localIdentity().also { live() }
    }
    override fun discover(timeoutMs: Long): ConnectionIdentity = current().discover(timeoutMs)
    override fun ready(peer: ConnectionIdentity, readinessTimeoutMs: Long) = current().ready(peer, readinessTimeoutMs)
    override fun authenticate(peer: ConnectionIdentity): ConnectionPeer = current().authenticate(peer)
    override fun negotiate(capacity: Int, timeoutMs: Long): Int = current().negotiate(capacity, timeoutMs)
    override fun io(wake: () -> Unit): ConnectionIo = current().io(wake)
    override fun close() {
        val resources = synchronized(gate) {
            if (closing) return
            closing = true
            delegate to accessory
        }
        closeConnectionResources(listOf(java.io.Closeable { resources.first?.close() }, java.io.Closeable { resources.second?.close() }))
        synchronized(gate) { closed = true }
    }
    override fun retired(): Boolean = synchronized(gate) {
        closed && !initializing && !uncertain && !failedLateClose &&
            accessory?.retired() != false && delegate?.retired() != false
    }
}
