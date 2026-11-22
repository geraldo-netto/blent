package com.blent.connection

import java.io.File
import java.io.OutputStream
import java.net.InetAddress
import java.net.Socket
import java.util.concurrent.CountDownLatch
import java.util.concurrent.FutureTask
import java.util.concurrent.Semaphore
import javax.net.ssl.SSLContext
import javax.net.ssl.SSLServerSocket
import javax.net.ssl.SSLSocket

/** One private loopback peer; ownership never reaches installed Blent sessions. */
internal class TunnelMeasurement(server: SSLContext, client: SSLContext, frames: Int, payloadBytes: Int) : AutoCloseable {
    private val resources = ArrayList<java.io.Closeable>(3)
    private val finish = CountDownLatch(1)
    private val accepted = CountDownLatch(1)
    private val peerWork = FutureTask { serve(frames, payloadBytes) }
    private val peer = Thread(peerWork, "blent-bench-peer")
    private val listener = (server.serverSocketFactory.createServerSocket(0, 1, InetAddress.getLoopbackAddress()) as SSLServerSocket).also { resources.add(it) }
    private val raw: Socket
    private val metered: MeteredSocket
    val socket: SSLSocket
    val records: Long get() = metered.meter.records
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
    private fun serve(frames: Int, payloadBytes: Int) {
        (listener.accept() as SSLSocket).use { connection ->
            connection.tcpNoDelay = true; connection.soTimeout = 5000
            try { connection.startHandshake() } finally { accepted.countDown() }
            val input = connection.inputStream
            val headerBytes = ByteArray(ConnectionWire.HEADER_BYTES)
            repeat(frames) { sequence ->
                check(input.readNBytes(headerBytes, 0, headerBytes.size) == headerBytes.size)
                val header = ConnectionWire.decode(headerBytes)
                check(header == ConnectionWire.Header(ConnectionWire.DATA, ConnectionWire.DISPLAY, 1, 1,
                    sequence.toLong(), payloadBytes, 0))
                val body = input.readNBytes(header.length)
                check(body.size == header.length && body.all { it == 7.toByte() })
                connection.outputStream.write(1)
            }
            finish.await() // Keep the peer alive until final CPU/record snapshots.
        }
    }
    fun finishPeer() { finish.countDown() }
    override fun close() {
        finishPeer()
        try { closeConnectionResources(resources.reversed()) }
        finally { peer.join() }
        peerWork.get()
    }
}

/** Candidate flush stays on the same native worker as its write. This benchmark
 * adapter is specific to socket streams; it does not change generic I/O semantics. */
internal class FlushingWrite(private val output: OutputStream) : OutputStream() {
    override fun write(value: Int) { output.write(value); output.flush() }
    override fun write(bytes: ByteArray, offset: Int, length: Int) { output.write(bytes, offset, length); output.flush() }
    override fun flush() = output.flush()
}

internal class MeasuredIo(private val connection: TunnelMeasurement, combined: Boolean) : AutoCloseable {
    private val origin = System.nanoTime()
    fun now(): Long = (System.nanoTime() - origin) / 1_000_000
    private val ready = Semaphore(0)
    val threads = ArrayList<Thread>(3)
    var completions = 0L; private set
    var polls = 0L; private set
    var waits = 0L; private set
    private val hints = java.util.concurrent.atomic.AtomicLong()
    val readyHints: Long get() = hints.get()
    private val io = StreamConnectionIo(connection.socket.inputStream,
        if (combined) FlushingWrite(connection.socket.outputStream) else connection.socket.outputStream,
        closer = java.io.Closeable { connection.socket.close() }, ready = { hints.incrementAndGet(); ready.release() },
        threadFactory = { name, task -> Thread(task, name).also { threads.add(it) } })
    fun perform(request: ConnectionIoRequest): Int {
        check(io.submit(request))
        while (true) {
            polls++
            val result = io.poll()
            if (result != null) {
                check(result.request === request)
                result.failure?.let { throw it }
                completions++
                return result.count
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
        io.stop()
        threads.forEach { it.join() }
        check(io.retired()); check(io.closeFailure() == null)
    }
}
