package com.blent.connection

import java.io.ByteArrayInputStream
import java.net.Socket
import java.net.SocketTimeoutException
import java.security.MessageDigest
import java.security.cert.CertificateFactory
import java.security.cert.X509Certificate
import java.util.concurrent.atomic.AtomicBoolean
import javax.net.ssl.SSLSocket

/** Stock socket adapter, called only by the supervised setup worker. Read clocks
 * are absolute; the independent owner also bounds blocked writes, TLS and close.
 * The optional lifetime retains the USB bridge beyond its exposed socket. */
internal class SocketConnectionSetup(
    private val socket: Socket,
    private val identity: () -> ClientTlsIdentity,
    private val lifetime: ConnectionSetupLifetime? = null,
    private val clock: () -> Long = System::nanoTime,
    private val secure: (ClientTlsIdentity, ByteArray, Socket) -> Socket = { local, peer, raw ->
        SecureConnection(local.keyManagers(), peer).connect(raw)
    },
) : ConnectionSetupBackend {
    private val gate = Any()
    private val closing = AtomicBoolean()
    private val closed = AtomicBoolean()
    private val lateCloseFailed = AtomicBoolean()
    private var loaded: ClientTlsIdentity? = null
    private var discovered: ByteArray? = null
    private var coordinated = false
    private var authenticated = false
    private var negotiated = false
    private var handedOff = false
    @Volatile private var transport = socket

    private fun live() { check(!closing.get()) { "Connection setup is closed" } }
    override fun localIdentity(): ConnectionIdentity {
        live()
        val local = loaded ?: identity().also { loaded = it }
        live()
        return local.publicIdentity()
    }
    private fun deadline(timeout: Long, maximum: Long = 60000): Long {
        require(timeout in 1..maximum)
        return clock() + timeout * 1000000
    }
    private fun remaining(end: Long): Int {
        live()
        val left = end - clock()
        if (left <= 0) throw SocketTimeoutException("Connection setup deadline")
        return ((left + 999999) / 1000000).coerceAtMost(Int.MAX_VALUE.toLong()).toInt()
    }
    private fun read(count: Int, end: Long): ByteArray {
        val bytes = ByteArray(count)
        var received = 0
        val input = transport.getInputStream()
        while (received < count) {
            transport.soTimeout = remaining(end)
            val amount = input.read(bytes, received, count - received)
            check(amount in 1..(count - received)) { "Truncated or invalid setup read" }
            received += amount
        }
        remaining(end)
        return bytes
    }
    private fun write(bytes: ByteArray, end: Long) {
        remaining(end)
        transport.getOutputStream().write(bytes)
        remaining(end)
    }
    private fun flush(end: Long) {
        remaining(end)
        transport.getOutputStream().flush()
        remaining(end)
    }
    private fun validateCertificate(bytes: ByteArray) {
        val input = ByteArrayInputStream(bytes)
        val certificate = CertificateFactory.getInstance("X.509").generateCertificate(input) as X509Certificate
        require(input.available() == 0) { "Trailing public certificate bytes" }
        certificate.checkValidity()
    }
    override fun discover(timeoutMs: Long): ConnectionIdentity {
        live(); check(discovered == null && !authenticated)
        val end = deadline(timeoutMs)
        val local = localIdentity().certificate()
        val header = read(ConnectionHello.HEADER_BYTES, end)
        val peer = read(ConnectionHello.decode(header, true), end)
        validateCertificate(peer)
        write(ConnectionHello.encode(false, local.size), end); write(local, end); flush(end)
        discovered = peer
        return ConnectionIdentity(peer)
    }
    private fun approved(peer: ConnectionIdentity): ByteArray {
        val expected = checkNotNull(discovered)
        require(peer.certificate().contentEquals(expected)) { "Approved public identity changed" }
        return expected
    }
    override fun ready(peer: ConnectionIdentity, readinessTimeoutMs: Long) {
        live(); check(!coordinated && !authenticated)
        approved(peer)
        val end = deadline(readinessTimeoutMs, 600000)
        write(ConnectionReady.encode(false), end); flush(end)
        ConnectionReady.decode(read(ConnectionReady.BYTES, end), true)
        coordinated = true
    }
    override fun authenticate(peer: ConnectionIdentity): ConnectionPeer {
        live(); check(coordinated && !authenticated)
        val expected = approved(peer)
        val secured = secure(checkNotNull(loaded), expected, socket)
        adopt(secured)
        val certificate = (secured as SSLSocket).session.peerCertificates.first().encoded
        val digest = MessageDigest.getInstance("SHA-256").digest(certificate)
        val proven = ConnectionPeer.parse(digest.joinToString("") { "%02x".format(it.toInt() and 255) })
        live(); authenticated = true
        return proven
    }
    private fun adopt(secured: Socket) {
        synchronized(gate) {
            if (!closing.get()) { transport = secured; return }
        }
        try { secured.close() } catch (failure: Exception) { lateCloseFailed.set(true); throw failure }
        error("TLS completed after setup cancellation")
    }
    override fun negotiate(capacity: Int, timeoutMs: Long): Int {
        live(); check(authenticated && !negotiated)
        val begun = clock()
        val end = deadline(timeoutMs)
        val window = ConnectionWindowNegotiation(capacity, 0, timeoutMs)
        val proposed = read(ConnectionWindow.BYTES, end)
        window.feed(proposed, 0, proposed.size, elapsed(begun))
        val loan = checkNotNull(window.output(elapsed(begun)))
        write(loan.bytes, end); window.wrote(loan, loan.length, elapsed(begun))
        val receipt = checkNotNull(window.flush(elapsed(begun)))
        flush(end)
        val selected = window.flushed(receipt, elapsed(begun))
        negotiated = true
        return selected
    }
    private fun elapsed(begun: Long): Long = (clock() - begun) / 1000000
    override fun io(wake: () -> Unit): ConnectionIo {
        live(); check(negotiated && !handedOff)
        handedOff = true
        transport.soTimeout = 0 // Idle frame reads are supervised separately.
        return StreamConnectionIo(transport.getInputStream(), transport.getOutputStream(), ::close, wake)
    }
    override fun close() {
        if (!closing.compareAndSet(false, true)) return
        val owned = synchronized(gate) { transport }
        var failure: Exception? = null
        try { owned.close() } catch (error: Exception) { failure = error }
        try { lifetime?.close() } catch (error: Exception) {
            if (failure == null) failure = error else failure.addSuppressed(error)
        }
        failure?.let { throw it }
        closed.set(true)
    }
    override fun retired(): Boolean = closed.get() && !lateCloseFailed.get() && lifetime?.retired() != false
}
