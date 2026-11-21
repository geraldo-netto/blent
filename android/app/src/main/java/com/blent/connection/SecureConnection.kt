package com.blent.connection

import android.os.Build
import java.io.ByteArrayInputStream
import java.net.Socket
import java.security.KeyStore
import java.security.MessageDigest
import java.security.cert.CertificateFactory
import java.security.cert.X509Certificate
import javax.net.ssl.KeyManager
import javax.net.ssl.SSLContext
import javax.net.ssl.SSLSocket
import javax.net.ssl.TrustManagerFactory

/** Standard mutual TLS adapter. Caller must obtain explicit or persisted trust
 * before supplying the peer certificate. Native socket work belongs in the
 * isolated connection worker, never on the UI thread. No trust-on-first-use. */
internal class SecureConnection(keys: Array<KeyManager>, approvedCertificate: ByteArray) {
    companion object {
        const val MAX_CERTIFICATE = 4096
        private const val PROTOCOL = "blent-usb/1"
        private const val HOSTNAME = "blent.local"

        private val PREFACE = byteArrayOf(66, 76, 69, 78, 84, 85, 83, 66, 0, 1)

        private fun certificate(bytes: ByteArray): X509Certificate {
            require(bytes.size in 1..MAX_CERTIFICATE)
            val input = ByteArrayInputStream(bytes)
            val certificate = CertificateFactory.getInstance("X.509").generateCertificate(input) as X509Certificate
            require(input.available() == 0) { "Trailing certificate bytes" }
            return certificate
        }

        private fun context(keys: Array<KeyManager>, peer: X509Certificate): SSLContext {
            val trust = KeyStore.getInstance(KeyStore.getDefaultType()).apply {
                load(null, null)
                setCertificateEntry("approved-peer", peer)
            }
            val managers = TrustManagerFactory.getInstance(TrustManagerFactory.getDefaultAlgorithm()).apply { init(trust) }
            return SSLContext.getInstance("TLS").apply { init(keys, managers.trustManagers, null) }
        }
    }

    private val peer = certificate(approvedCertificate)
    private val fingerprint = MessageDigest.getInstance("SHA-256").digest(peer.encoded)
    private val context = context(keys.copyOf(), peer)

    /** Takes socket ownership even on failure. Its supervisor must bound native
     * close and retain ownership until the worker has actually retired. */
    fun connect(socket: Socket, remaining: () -> Int): Socket {
        var owned = socket
        try {
            val readTimeout = socket.soTimeout
            val secured = context.socketFactory.createSocket(socket, HOSTNAME, socket.port, true) as SSLSocket
            owned = secured
            configure(secured, timeout(remaining))
            secured.startHandshake()
            verify(secured)
            protocol(secured, remaining)
            secured.soTimeout = readTimeout
            return secured
        } catch (failure: Exception) {
            runCatching { owned.close() }
            throw failure
        }
    }

    private fun configure(socket: SSLSocket, timeoutMs: Int) {
        socket.useClientMode = true
        socket.soTimeout = timeoutMs
        socket.enabledProtocols = socket.supportedProtocols.filter { it == "TLSv1.3" || it == "TLSv1.2" }.toTypedArray()
        socket.sslParameters = socket.sslParameters.apply {
            endpointIdentificationAlgorithm = "HTTPS"
            // API 27/28 use the authenticated preface without public ALPN calls.
            if (Build.VERSION.SDK_INT >= 29) applicationProtocols = arrayOf(PROTOCOL)
        }
    }

    private fun timeout(remaining: () -> Int): Int = remaining().also {
        require(it in 1..60000) { "TLS setup timeout must be within 1–60000 ms" }
    }

    private fun protocol(socket: SSLSocket, remaining: () -> Int) {
        timeout(remaining)
        socket.outputStream.write(PREFACE)
        socket.outputStream.flush()
        check(receive(socket, remaining).contentEquals(PREFACE)) { "Unexpected authenticated protocol preface" }
    }

    private fun receive(socket: SSLSocket, remaining: () -> Int): ByteArray {
        val received = ByteArray(PREFACE.size)
        val input = socket.inputStream
        var count = 0
        while (count < received.size) {
            socket.soTimeout = timeout(remaining)
            val amount = input.read(received, count, received.size - count)
            check(amount in 1..(received.size - count)) { "Truncated authenticated protocol preface" }
            count += amount
        }
        timeout(remaining)
        return received
    }

    private fun verify(socket: SSLSocket) {
        if (Build.VERSION.SDK_INT >= 29) check(socket.applicationProtocol == PROTOCOL) { "Connection protocol not negotiated" }
        val leaf = socket.session.peerCertificates.firstOrNull() ?: error("Missing authenticated peer")
        val observed = MessageDigest.getInstance("SHA-256").digest(leaf.encoded)
        check(MessageDigest.isEqual(fingerprint, observed)) { "Authenticated peer changed" }
    }
}
