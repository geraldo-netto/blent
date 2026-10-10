package com.blent.connection

import java.io.Closeable
import java.io.InputStream
import java.io.OutputStream
import java.net.InetAddress
import java.net.InetSocketAddress
import java.net.ServerSocket
import java.net.Socket
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicBoolean
import java.util.concurrent.atomic.AtomicReference

/** One bridge admission per physical connection owner, including failed setup.
 * Rejected repeat admission leaves the supplied streams with their caller. */
internal class SocketBridgeAdmission(
    private val sockets: () -> SocketBridgeSockets = { SocketBridgeSockets.open() },
    private val threads: (String, Runnable) -> Thread = { name, task -> Thread(task, name) },
) {
    private val admitted = AtomicBoolean()
    fun open(input: InputStream, output: OutputStream): SocketBridge {
        check(admitted.compareAndSet(false, true)) { "Connection bridge already admitted" }
        var pair: SocketBridgeSockets? = null
        try {
            pair = sockets()
            pair.verify()
            return SocketBridge(input, output, pair, threads)
        } catch (failure: Exception) {
            closeConnectionFailure(failure, listOfNotNull(pair, input, output))
            throw failure
        }
    }
}

/** Only a connected pair from the same private listener can enter the bridge. */
internal class SocketBridgeSockets(val client: Socket, val relay: Socket) : Closeable {
    companion object {
        fun open(
            listener: ServerSocket = ServerSocket(),
            clientFactory: () -> Socket = { Socket() },
        ): SocketBridgeSockets {
            var client: Socket? = null
            var relay: Socket? = null
            try {
                return listener.use {
                    listener.bind(InetSocketAddress(InetAddress.getLoopbackAddress(), 0), 1)
                    listener.soTimeout = 3000
                    client = clientFactory()
                    client!!.connect(listener.localSocketAddress, 3000)
                    client!!.tcpNoDelay = true
                    relay = listener.accept()
                    relay!!.tcpNoDelay = true
                    SocketBridgeSockets(client!!, relay!!)
                }
            } catch (failure: Exception) {
                closeConnectionFailure(failure, listOfNotNull(client, relay))
                throw failure
            }
        }
    }
    fun verify() {
        check(client.isConnected && relay.isConnected)
        check(client.localAddress.isLoopbackAddress && relay.localAddress.isLoopbackAddress)
        check(relay.remoteSocketAddress == client.localSocketAddress)
        check(relay.localSocketAddress == client.remoteSocketAddress) { "Loopback peer changed" }
    }
    override fun close() = closeConnectionResources(listOf(client, relay))
}

/** Byte-stream adapter for stock socket-based TLS. No framing or authority here.
 * open/start/close/await belong off the UI thread in a supervised private process.
 * A terminal pump only requests retirement: its peer may still be blocked in
 * native I/O. The supervisor owns absolute deadlines and process termination.
 * close() can itself block; retired() includes its completion and actual thread
 * exit. Keep this handle on startup failure until confirmed retirement. */
internal class SocketBridge internal constructor(
    private val input: InputStream,
    private val output: OutputStream,
    private val sockets: SocketBridgeSockets,
    private val threads: (String, Runnable) -> Thread,
) : Closeable {
    companion object { const val BUFFER_BYTES = 65536 }
    private val started = AtomicBoolean()
    private val stopping = AtomicBoolean()
    private val closing = AtomicBoolean()
    private val closed = AtomicBoolean()
    private val terminal = CountDownLatch(1)
    private val failure = AtomicReference<Throwable?>()
    private val outputEof = AtomicBoolean()
    @Volatile private var workers: List<Thread> = emptyList()
    @Volatile private var startupFinished = false
    val socket: Socket get() = sockets.client

    fun start() {
        check(started.compareAndSet(false, true)) { "Bridge already started" }
        try {
            check(!stopping.get()) { "Bridge already stopped" }
            val relayOutput = sockets.relay.getOutputStream()
            val relayInput = sockets.relay.getInputStream()
            val incoming = threads("blent-accessory-input", Runnable { pump(input, relayOutput, false) })
            val outgoing = threads("blent-accessory-output", Runnable { pump(relayInput, output, true) })
            workers = listOf(incoming, outgoing)
            for (worker in workers) worker.start()
        } catch (error: Exception) {
            failure.compareAndSet(null, error)
            requestStop()
            throw error
        } finally { startupFinished = true }
    }

    private fun pump(source: InputStream, destination: OutputStream, outgoing: Boolean) {
        try {
            val buffer = ByteArray(BUFFER_BYTES)
            while (!stopping.get()) {
                val count = source.read(buffer, 0, buffer.size)
                if (count == -1) { if (outgoing) outputEof.set(true); break }
                check(count in 0..buffer.size) { "Invalid stream read count" }
                if (count == 0) { Thread.sleep(1); continue } // Valid USB zero-length transfer.
                destination.write(buffer, 0, count)
                destination.flush()
            }
        } catch (error: Exception) {
            failure.compareAndSet(null, error)
        } finally { requestStop() }
    }

    /** Call after closing the TLS client socket. Success requires relay EOF after
     * all queued bytes were written, actual outgoing thread exit, and no failure.
     * Incoming EOF or a stop request alone cannot establish a graceful drain. */
    fun awaitOutputRetired(timeoutMillis: Long): Boolean {
        require(timeoutMillis in 0..60000)
        val outgoing = workers.getOrNull(1) ?: return false
        if (timeoutMillis > 0) outgoing.join(timeoutMillis)
        return !outgoing.isAlive && outputEof.get() && failure.get() == null
    }

    fun requestStop() { stopping.set(true); terminal.countDown() }
    fun failure(): Throwable? = failure.get()
    fun awaitTerminal(timeoutMillis: Long): Boolean {
        require(timeoutMillis in 0..60000)
        return terminal.await(timeoutMillis, TimeUnit.MILLISECONDS)
    }
    fun retired(): Boolean = (!started.get() || startupFinished) && closed.get() && workers.none { it.isAlive }

    override fun close() {
        requestStop()
        if (!closing.compareAndSet(false, true)) return
        try { closeConnectionResources(listOf(sockets, input, output)); closed.set(true) }
        catch (error: Exception) { failure.compareAndSet(null, error); throw error }
    }
}

/** Preserve the original error while attempting every acquired resource. */
internal fun closeConnectionFailure(failure: Throwable, resources: List<Closeable>) {
    for (resource in resources) try { resource.close() } catch (error: Exception) { failure.addSuppressed(error) }
}
internal fun closeConnectionResources(resources: List<Closeable>) {
    var failure: Exception? = null
    for (resource in resources) {
        try { resource.close() } catch (error: Exception) {
            if (failure == null) failure = error else failure.addSuppressed(error)
        }
    }
    failure?.let { throw it }
}
