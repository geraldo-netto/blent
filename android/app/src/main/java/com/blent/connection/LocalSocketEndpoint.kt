package com.blent.connection

import java.io.Closeable
import java.net.InetAddress
import java.net.InetSocketAddress
import java.net.ServerSocket
import java.net.Socket

/** OS adapter for one locally allocated IPv4 loopback listener. No remotely
 * supplied host or port is representable. Native owner supervises these blocking
 * methods; every partial/late handle remains reachable through this object. */
internal class LocalSocketEndpoint(
    private val wake: () -> Unit = {},
    private val listener: () -> ServerSocket = { ServerSocket() },
    private val ioFactory: (Socket) -> ConnectionIo = { socket ->
        StreamConnectionIo(socket.getInputStream(), socket.getOutputStream(), socket, wake)
    },
) : LocalEndpointNative {
    private val gate = Any()
    private var server: ServerSocket? = null
    private var socket: Socket? = null
    private var io: ConnectionIo? = null
    private var started = false
    private var accepting = false
    private var busy = false
    private var uncertain = false
    private var closing = false
    private var closed = false
    private var lateCloseFailed = false

    override fun bind(): Int {
        synchronized(gate) {
            check(!started && !closing)
            started = true; uncertain = true; busy = true
        }
        try {
            val bound = listener()
            adopt(bound)
            bound.reuseAddress = false
            bound.bind(InetSocketAddress(InetAddress.getByAddress(byteArrayOf(127, 0, 0, 1)), 0), 1)
            synchronized(gate) { check(!closing) }
            return bound.localPort
        } finally { synchronized(gate) { busy = false } }
    }

    private fun adopt(bound: ServerSocket) {
        val late = synchronized(gate) { server = bound; uncertain = false; closing }
        if (late) lateClose(bound)
    }
    private fun adopt(accepted: Socket) {
        val late = synchronized(gate) { socket = accepted; closing }
        if (late) lateClose(accepted)
    }
    private fun lateClose(value: Closeable): Nothing {
        try { value.close() } catch (failure: Exception) { synchronized(gate) { lateCloseFailed = true }; throw failure }
        error("Local endpoint completed after cancellation")
    }
    override fun accept(): ConnectionIo {
        val bound = synchronized(gate) {
            check(!closing && !accepting && !busy)
            val value = checkNotNull(server)
            accepting = true; busy = true
            value
        }
        try {
            val accepted = bound.accept()
            adopt(accepted)
            check(accepted.inetAddress.isLoopbackAddress) { "Local endpoint peer is not loopback" }
            accepted.tcpNoDelay = true
            synchronized(gate) { uncertain = true }
            val created = ioFactory(accepted)
            val late = synchronized(gate) { io = created; uncertain = false; closing }
            if (late) lateClose(Closeable { created.stop() })
            return created
        } finally { synchronized(gate) { busy = false } }
    }
    override fun close() {
        val resources = synchronized(gate) {
            if (closing) return
            closing = true
            val ownedIo = io
            listOfNotNull(Closeable { ownedIo?.stop() }, socket, server)
        }
        closeConnectionResources(resources)
        synchronized(gate) { closed = true }
    }
    override fun retired(): Boolean = synchronized(gate) {
        closed && !busy && !uncertain && !lateCloseFailed && io?.retired() != false && io?.closeFailure() == null
    }
}
