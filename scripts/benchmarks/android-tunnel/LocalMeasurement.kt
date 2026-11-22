package com.blent.connection

import java.io.Closeable
import java.net.InetAddress
import java.net.InetSocketAddress
import java.net.ServerSocket
import java.net.Socket
import java.util.concurrent.CountDownLatch
import java.util.concurrent.FutureTask

/** T1763 private TCP analogue for stream coordination, not an Android LocalSocket. */
internal class LocalMeasurement(frames: Int, payloadBytes: Int) : MeasurementConnection {
    init { require(frames in 1..60 && payloadBytes in 1..ConnectionWire.MAX_PAYLOAD) }
    private val resources = ArrayList<Closeable>(2)
    private val finish = CountDownLatch(1)
    private val listener = ServerSocket(0, 1, InetAddress.getLoopbackAddress()).also { resources.add(it) }
    private val work = FutureTask { serve(frames, payloadBytes) }
    private val peer = Thread(work, "blent-local-bench-peer")
    val socket: Socket
    override val input: java.io.InputStream get() = socket.inputStream
    override val output: java.io.OutputStream get() = socket.outputStream
    override fun closeStream() = socket.close()
    init {
        try {
            listener.soTimeout = 5000
            peer.start()
            socket = Socket().also {
                resources.add(it)
                it.connect(InetSocketAddress(InetAddress.getLoopbackAddress(), listener.localPort), 5000)
                it.tcpNoDelay = true; it.soTimeout = 5000
            }
        } catch (error: Exception) {
            try { closeConnectionResources(resources.reversed()) }
            catch (cleanup: Exception) { error.addSuppressed(cleanup) }
            finally { finishPeer(); peer.join() }
            throw error
        }
    }
    private fun serve(frames: Int, payloadBytes: Int) {
        listener.accept().use { connection ->
            connection.tcpNoDelay = true; connection.soTimeout = 5000
            repeat(frames) {
                val bytes = connection.inputStream.readNBytes(payloadBytes)
                check(bytes.size == payloadBytes && bytes.all { it == 7.toByte() }) { "T1763 local payload mismatch" }
                connection.outputStream.write(1)
            }
            finish.await()
        }
    }
    override fun finishPeer() { finish.countDown() }
    override fun close() {
        finishPeer()
        try { closeConnectionResources(resources.reversed()) }
        finally { peer.join() }
        work.get()
    }
}

/** One exact ring receipt; unsupported endpoint operations fail the fixture. */
internal class LocalMeasurementEndpoint : ConnectionEndpoints {
    val lease = ConnectionResourceLease(ConnectionTicket(1, ConnectionPeer.parse("ab".repeat(32))),
        ConnectionManagement.Key(ConnectionManagement.Resource.DISPLAY, 1, 1), 1)
    private var pending: ConnectionChannelRead? = null
    private var borrowed: ConnectionChannelRead? = null
    fun finished(): Boolean = pending == null && borrowed == null
    fun prepare(bytes: ByteArray, offset: Int, length: Int) {
        check(finished())
        pending = ConnectionChannelRead(bytes, offset, length)
    }
    override fun read(lease: ConnectionResourceLease, channel: Int, maxBytes: Int): ConnectionChannelRead {
        check(lease === this.lease && channel == ConnectionWire.CONTROL)
        check(borrowed == null)
        val next = checkNotNull(pending)
        check(next.length <= maxBytes)
        pending = null; borrowed = next
        return next
    }
    override fun consumed(lease: ConnectionResourceLease, channel: Int, receipt: ConnectionChannelRead) {
        check(lease === this.lease && channel == ConnectionWire.CONTROL)
        check(borrowed === receipt)
        borrowed = null
    }
    override fun nextEvent(): ConnectionSessionEvent? = error("Unexpected endpoint event request")
    override fun prepared(lease: ConnectionResourceLease, now: Long) = error("Unexpected preparation")
    override fun permissionRequired(lease: ConnectionResourceLease, now: Long) = error("Unexpected permission")
    override fun reject(lease: ConnectionResourceLease, outcome: ConnectionManagement.Outcome) = error("Unexpected rejection")
    override fun cancel(lease: ConnectionResourceLease, reason: ConnectionManagement.Reason) = error("Unexpected cancellation")
    override fun current(lease: ConnectionResourceLease): Boolean = error("Unexpected admission query")
    override fun offer(lease: ConnectionResourceLease, channel: Int, bytes: ByteArray): Boolean = error("Unexpected producer")
    override fun nativeRetired(lease: ConnectionResourceLease) = error("Whole-resource retirement belongs to outer owner")
}
