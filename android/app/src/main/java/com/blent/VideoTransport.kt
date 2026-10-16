package com.blent

import java.io.InputStream
import java.net.InetSocketAddress
import java.net.Socket

/** Owns a published socket, including a connection blocked in connect/read.
 * Retirement always names its socket so an old worker cannot close a new run. */
internal class VideoTransport(
    private val monitor: Any,
    private val current: (Long) -> Boolean,
    private val createSocket: () -> Socket,
) {
    private var socket: Socket? = null

    fun newConnection(): Socket = createSocket()

    fun connect(generation: Long, connection: Socket, routes: DisplayRoutes): InputStream? {
        synchronized(monitor) {
            if (!current(generation)) return null
            socket = connection
        }
        connection.connect(InetSocketAddress("127.0.0.1", routes.videoPort), 5000)
        connection.apply {
            tcpNoDelay = true
            soTimeout = 10000
            // Expose backpressure rather than buffer seconds of stale video.
            receiveBufferSize = 128 * 1024
        }
        val input = connection.getInputStream()
        synchronized(monitor) {
            if (!current(generation)) return null
        }
        return input
    }

    fun open(generation: Long, connection: Socket, routes: DisplayRoutes, liveness: Boolean): InputStream? {
        val input = connect(generation, connection, routes) ?: return null
        routes.token?.let { token ->
            connection.getOutputStream().apply { write(token.toByteArray(Charsets.US_ASCII)); flush() }
        }
        if (!acknowledge(generation, connection, liveness)) return null
        return input
    }
    fun acknowledge(generation: Long, connection: Socket, enabled: Boolean): Boolean {
        if (!owns(generation, connection)) return false
        if (enabled) {
            connection.getOutputStream().apply { write(VideoNegotiation.acknowledgement()); flush() }
        }
        return owns(generation, connection)
    }
    private fun owns(generation: Long, connection: Socket): Boolean = synchronized(monitor) {
        current(generation) && socket === connection
    }

    fun retire(connection: Socket?) {
        try { connection?.close() } catch (_: Exception) {}
        synchronized(monitor) {
            if (socket === connection) socket = null
        }
    }

    // Caller owns the monitor for the whole codec/generation handoff.
    fun interrupt() {
        try { socket?.close() } catch (_: Exception) {}
    }

    fun stop() {
        interrupt()
        socket = null
    }
}
