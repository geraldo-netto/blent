package com.blent.connection

import java.net.InetAddress
import java.net.Socket
import java.security.KeyFactory
import java.security.KeyStore
import java.security.cert.CertificateFactory
import java.security.interfaces.ECPrivateKey
import java.security.spec.PKCS8EncodedKeySpec
import java.util.concurrent.Executors
import java.util.concurrent.TimeUnit
import javax.net.ssl.*
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [34])
class IdentityTlsTest {
    private fun fixture(name: String): ByteArray = javaClass.classLoader!!
        .getResourceAsStream("connection-tls/$name")!!.use { it.readBytes() }
    private class OpaqueKey(private val source: ECPrivateKey) : ECPrivateKey by source {
        override fun getEncoded(): ByteArray? = null
        override fun getFormat(): String? = null
    }
    private fun entry(name: String, opaque: Boolean): KeyStore.PrivateKeyEntry {
        val key = KeyFactory.getInstance("EC").generatePrivate(PKCS8EncodedKeySpec(fixture("$name.pk8"))) as ECPrivateKey
        val certificate = CertificateFactory.getInstance("X.509").generateCertificate(fixture("$name.der").inputStream())
        return KeyStore.PrivateKeyEntry(if (opaque) OpaqueKey(key) else key, arrayOf(certificate))
    }
    private fun server(): SSLContext {
        val host = entry("host", false)
        val store = KeyStore.getInstance(KeyStore.getDefaultType()).apply {
            load(null, null)
            setKeyEntry("host", host.privateKey, charArrayOf(), host.certificateChain)
        }
        val trust = KeyStore.getInstance(KeyStore.getDefaultType()).apply {
            load(null, null); setCertificateEntry("client", entry("tablet", false).certificate)
        }
        val keys = KeyManagerFactory.getInstance(KeyManagerFactory.getDefaultAlgorithm()).apply { init(store, charArrayOf()) }
        val peers = TrustManagerFactory.getInstance(TrustManagerFactory.getDefaultAlgorithm()).apply { init(trust) }
        return SSLContext.getInstance("TLS").apply { init(keys.keyManagers, peers.trustManagers, null) }
    }
    private fun transfer(protocol: String, identity: ClientTlsIdentity) {
        val listener = server().serverSocketFactory.createServerSocket(0, 1, InetAddress.getLoopbackAddress()) as SSLServerSocket
        listener.soTimeout = 3000; listener.needClientAuth = true; listener.enabledProtocols = arrayOf(protocol)
        listener.sslParameters = listener.sslParameters.apply { applicationProtocols = arrayOf("blent-usb/1") }
        val executor = Executors.newSingleThreadExecutor()
        val peer = executor.submit<Unit> {
            listener.accept().use { raw ->
                val socket = raw as SSLSocket
                socket.soTimeout = 3000; socket.startHandshake()
                assertArrayEquals(identity.publicIdentity().certificate(), socket.session.peerCertificates[0].encoded)
                val preface = ByteArray(10)
                java.io.DataInputStream(socket.inputStream).readFully(preface)
                assertArrayEquals(fixture("protocol-v1.bin"), preface)
                socket.outputStream.write(preface); socket.outputStream.flush()
                assertEquals(42, socket.inputStream.read())
            }
        }
        try {
            val raw = Socket(InetAddress.getLoopbackAddress(), listener.localPort)
            SecureConnection(identity.keyManagers(), fixture("host.der")).connect(raw).use {
                it.getOutputStream().write(42); it.getOutputStream().flush()
            }
            peer.get(5, TimeUnit.SECONDS)
        } finally {
            listener.close(); executor.shutdownNow(); assertTrue(executor.awaitTermination(5, TimeUnit.SECONDS))
        }
    }
    @Test fun t846OpaqueClientIdentityWorksWithMaintainedTlsAdapter() {
        val directory = java.nio.file.Files.createTempDirectory("blent-identity-tls").toFile()
        val store = object : IdentityStore {
            override fun contains(alias: String): Boolean = true
            override fun read(alias: String): KeyStore.PrivateKeyEntry = entry("tablet", true)
        }
        try {
            val identity = AndroidConnectionIdentity(directory, store, IdentityGenerator { fail("Unexpected generation") }).load()
            for (protocol in listOf("TLSv1.2", "TLSv1.3")) transfer(protocol, identity)
        } finally { directory.deleteRecursively() }
    }
}
