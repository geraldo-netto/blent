package com.blent.connection

import java.io.File
import java.net.InetAddress
import java.net.Socket
import java.util.concurrent.CountDownLatch
import java.util.concurrent.FutureTask
import java.util.concurrent.Semaphore
import javax.net.ssl.SSLContext
import javax.net.ssl.SSLServerSocket
import javax.net.ssl.SSLSocket

/** One private loopback peer; ownership never reaches installed Blent sessions. */
internal interface MeasurementConnection : AutoCloseable {
    val input: java.io.InputStream
    val output: java.io.OutputStream
    fun closeStream()
    fun finishPeer()
}

internal class TunnelMeasurement(server: SSLContext, client: SSLContext,
                                 private val exchange: (SSLSocket) -> Unit) : MeasurementConnection {
    private val resources = ArrayList<java.io.Closeable>(3)
    private val finish = CountDownLatch(1)
    private val accepted = CountDownLatch(1)
    private val peerWork = FutureTask { serve() }
    private val peer = Thread(peerWork, "blent-bench-peer")
    private val listener = (server.serverSocketFactory.createServerSocket(0, 1, InetAddress.getLoopbackAddress()) as SSLServerSocket).also { resources.add(it) }
    private val raw: Socket
    private val metered: MeteredSocket
    val socket: SSLSocket
    val records: Long get() = metered.meter.records
    override val input: java.io.InputStream get() = socket.inputStream
    override val output: java.io.OutputStream get() = socket.outputStream
    override fun closeStream() = socket.close()
    init {
        try {
            listener.needClientAuth = true
            listener.enabledProtocols = arrayOf("TLSv1.3")
            listener.soTimeout = 5000 // Private connection setup protocol deadline.
            peer.start()
            raw = Socket().also {
                resources.add(it)
                it.connect(java.net.InetSocketAddress(InetAddress.getLoopbackAddress(), listener.localPort), 5000)
                it.tcpNoDelay = true; it.soTimeout = 5000
            }
            metered = MeteredSocket(raw)
            socket = (client.socketFactory.createSocket(metered, "blent.local", listener.localPort, true) as SSLSocket).also { resources.add(it) }
            socket.enabledProtocols = arrayOf("TLSv1.3")
            socket.startHandshake()
            check(socket.session.protocol == "TLSv1.3")
            accepted.await()
            if (peerWork.isDone) peerWork.get()
        } catch (error: Exception) {
            try { closeConnectionResources(resources.reversed()) }
            catch (cleanup: Exception) { error.addSuppressed(cleanup) }
            finally { finish.countDown(); peer.join() }
            throw error
        }
    }
    private fun serve() {
        (listener.accept() as SSLSocket).use { connection ->
            connection.tcpNoDelay = true; connection.soTimeout = 5000
            try { connection.startHandshake() } finally { accepted.countDown() }
            exchange(connection)
            finish.await() // Keep the peer alive until final CPU/record snapshots.
        }
    }
    override fun finishPeer() { finish.countDown() }
    override fun close() {
        finishPeer()
        try { closeConnectionResources(resources.reversed()) }
        finally { peer.join() }
        peerWork.get()
    }
}

internal class MeasuredIo(private val connection: MeasurementConnection) : AutoCloseable {
    private val origin = System.nanoTime()
    fun now(): Long = (System.nanoTime() - origin) / 1_000_000
    private val ready = Semaphore(0)
    val threads = ArrayList<Thread>(3)
    var completions = 0L; private set
    var polls = 0L; private set
    var waits = 0L; private set
    private val hints = java.util.concurrent.atomic.AtomicLong()
    val readyHints: Long get() = hints.get()
    val port: ConnectionIo = StreamConnectionIo(connection.input,
        connection.output,
        closer = java.io.Closeable { connection.closeStream() }, ready = { hints.incrementAndGet(); ready.release() },
        threadFactory = { name, task -> Thread(task, name).also { threads.add(it) } })
    fun perform(request: ConnectionIoRequest): Int {
        check(port.submit(request))
        val result = completion()
        check(result.request === request)
        result.failure?.let { throw it }
        return result.count
    }
    fun completion(): ConnectionIoCompletion {
        while (true) {
            polls++
            val result = port.poll()
            if (result != null) {
                completions++
                return result
            }
            waits++
            ready.acquire()
        }
    }
    fun acknowledgement() {
        val value = ByteArray(1)
        check(perform(ConnectionIoRequest.Read(ConnectionBuffer(value, 0, 1))) == 1 && value[0] == 1.toByte())
    }
    override fun close() {
        connection.finishPeer()
        port.stop()
        threads.forEach { it.join() }
        check(port.retired()); check(port.closeFailure() == null)
    }
}
