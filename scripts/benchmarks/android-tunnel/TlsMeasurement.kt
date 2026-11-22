package com.blent.connection

import java.io.File
import java.io.InputStream
import java.io.OutputStream
import java.net.InetAddress
import java.net.Socket
import java.security.KeyFactory
import java.security.KeyStore
import java.security.cert.CertificateFactory
import java.security.cert.X509Certificate
import java.security.spec.PKCS8EncodedKeySpec
import javax.net.ssl.KeyManagerFactory
import javax.net.ssl.SSLContext
import javax.net.ssl.TrustManagerFactory

/** Test identities are public fixtures; this context never enters an app build. */
internal fun measurementTls(fixtures: File, own: String, peer: String): SSLContext {
    val certificate = File(fixtures, "$own.der").inputStream().use {
        CertificateFactory.getInstance("X.509").generateCertificate(it) as X509Certificate
    }
    val key = KeyFactory.getInstance(certificate.publicKey.algorithm)
        .generatePrivate(PKCS8EncodedKeySpec(File(fixtures, "$own.pk8").readBytes()))
    val password = "public-test-fixture".toCharArray()
    val keys = KeyStore.getInstance(KeyStore.getDefaultType()).apply {
        load(null); setKeyEntry("own", key, password, arrayOf(certificate))
    }
    val trust = KeyStore.getInstance(KeyStore.getDefaultType()).apply {
        load(null)
        File(fixtures, "$peer.der").inputStream().use {
            setCertificateEntry("peer", CertificateFactory.getInstance("X.509").generateCertificate(it))
        }
    }
    val managers = KeyManagerFactory.getInstance(KeyManagerFactory.getDefaultAlgorithm()).apply { init(keys, password) }
    val trusted = TrustManagerFactory.getInstance(TrustManagerFactory.getDefaultAlgorithm()).apply { init(trust) }
    return SSLContext.getInstance("TLSv1.3").apply { init(managers.keyManagers, trusted.trustManagers, null) }
}

/** One stream owner feeds this fixed five-byte parser only after successful I/O.
 * Snapshots follow exact native completion; no payload or socket is retained. */
internal class TlsRecordCounter {
    private val header = ByteArray(5)
    private var used = 0
    private var remaining = 0
    var records = 0L; private set
    fun accept(bytes: ByteArray, offset: Int, length: Int) {
        java.util.Objects.checkFromIndexSize(offset, length, bytes.size)
        var cursor = offset
        val end = offset + length
        while (cursor < end) {
            val consumed = if (remaining > 0) payload(end - cursor) else prefix(bytes, cursor, end - cursor)
            cursor += consumed
        }
    }
    private fun payload(available: Int): Int {
        val count = minOf(available, remaining)
        remaining -= count
        return count
    }
    private fun prefix(bytes: ByteArray, offset: Int, length: Int): Int {
        val count = minOf(length, header.size - used)
        bytes.copyInto(header, used, offset, offset + count)
        used += count
        if (used == header.size) {
            remaining = ((header[3].toInt() and 255) shl 8) or (header[4].toInt() and 255)
            check(remaining <= 18432) { "TLS record exceeds measurement bound" }
            if (header[0].toInt() == 23) records++
            used = 0
        }
        return count
    }
}

internal class RecordMeter(private val output: OutputStream) : OutputStream() {
    private val counter = TlsRecordCounter()
    val records: Long get() = counter.records
    override fun write(value: Int) = write(byteArrayOf(value.toByte()), 0, 1)
    override fun write(bytes: ByteArray, offset: Int, length: Int) {
        output.write(bytes, offset, length)
        counter.accept(bytes, offset, length)
    }
    override fun flush() = output.flush()
    override fun close() = output.close()
}

internal class RecordInputMeter(private val input: InputStream) : InputStream() {
    private val counter = TlsRecordCounter()
    private val single = ByteArray(1)
    val records: Long get() = counter.records
    override fun read(): Int {
        val value = input.read()
        if (value >= 0) {
            single[0] = value.toByte()
            counter.accept(single, 0, 1)
        }
        return value
    }
    override fun read(bytes: ByteArray, offset: Int, length: Int): Int {
        java.util.Objects.checkFromIndexSize(offset, length, bytes.size)
        val count = input.read(bytes, offset, length)
        if (count > 0) counter.accept(bytes, offset, count)
        return count
    }
    override fun available(): Int = input.available()
    override fun close() = input.close()
}

/** Layered JSSE uses this socket's original streams; no TCP relay or extra queue. */
internal class MeteredSocket(private val original: Socket) : Socket() {
    val meter = RecordMeter(original.getOutputStream())
    val incoming = RecordInputMeter(original.getInputStream())
    override fun getInputStream(): InputStream = incoming
    override fun getOutputStream(): OutputStream = meter
    override fun getInetAddress(): InetAddress = original.inetAddress
    override fun getLocalAddress(): InetAddress = original.localAddress
    override fun getPort(): Int = original.port
    override fun getLocalPort(): Int = original.localPort
    override fun isConnected(): Boolean = original.isConnected
    override fun isBound(): Boolean = original.isBound
    override fun isClosed(): Boolean = original.isClosed
    override fun setSoTimeout(value: Int) { original.soTimeout = value }
    override fun getSoTimeout(): Int = original.soTimeout
    override fun setTcpNoDelay(value: Boolean) { original.tcpNoDelay = value }
    override fun getTcpNoDelay(): Boolean = original.tcpNoDelay
    override fun close() {
        try { original.close() } finally { super.close() }
    }
}
