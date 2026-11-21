package com.blent.connection

import java.net.InetAddress
import java.net.ServerSocket
import java.net.Socket
import java.security.KeyFactory
import java.security.KeyStore
import java.security.cert.CertificateFactory
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
class SecureConnectionTest {
    private fun fixture(name: String): ByteArray = javaClass.classLoader!!
        .getResourceAsStream("connection-tls/$name")!!.use { it.readBytes() }
    private fun certificate(name: String) = CertificateFactory.getInstance("X.509")
        .generateCertificate(fixture("$name.der").inputStream())
    private fun keys(name: String): Array<KeyManager> {
        val key = KeyFactory.getInstance("EC").generatePrivate(PKCS8EncodedKeySpec(fixture("$name.pk8")))
        val store = KeyStore.getInstance(KeyStore.getDefaultType()).apply {
            load(null, null)
            setKeyEntry("identity", key, charArrayOf(), arrayOf(certificate(name)))
        }
        return KeyManagerFactory.getInstance(KeyManagerFactory.getDefaultAlgorithm()).apply {
            init(store, charArrayOf())
        }.keyManagers
    }
    private fun serverContext(peer: String): SSLContext {
        val trust = KeyStore.getInstance(KeyStore.getDefaultType()).apply {
            load(null, null); setCertificateEntry("peer", certificate(peer))
        }
        val managers = TrustManagerFactory.getInstance(TrustManagerFactory.getDefaultAlgorithm()).apply { init(trust) }
        return SSLContext.getInstance("TLS").apply { init(keys("host"), managers.trustManagers, null) }
    }
    private fun trial(clientPeer: String = "host", serverPeer: String = "tablet", protocol: String? = "blent-usb/1", readTimeout: Int = 0, preface: ByteArray = fixture("protocol-v1.bin"), sendPreface: Boolean = true, tls12: Boolean = false, setupTimeout: Int = 3000, remaining: () -> Int = { setupTimeout }): Boolean {
        val listener = serverContext(serverPeer).serverSocketFactory.createServerSocket(0, 1, InetAddress.getLoopbackAddress()) as SSLServerSocket
        listener.soTimeout = 3000
        listener.needClientAuth = true
        if (tls12) listener.enabledProtocols = arrayOf("TLSv1.2")
        listener.sslParameters = listener.sslParameters.apply { applicationProtocols = if (protocol == null) emptyArray() else arrayOf(protocol) }
        val executor = Executors.newSingleThreadExecutor()
        val server = executor.submit<Boolean> {
            runCatching { listener.accept().use { raw ->
                val socket = raw as SSLSocket
                socket.soTimeout = 3000
                socket.startHandshake()
                if (sendPreface) {
                    socket.outputStream.write(preface); socket.outputStream.flush()
                    val observed = ByteArray(10)
                    java.io.DataInputStream(socket.inputStream).readFully(observed)
                    assertArrayEquals(fixture("protocol-v1.bin"), observed)
                }
                assertEquals(42, socket.inputStream.read())
                socket.outputStream.write(43); socket.outputStream.flush()
            } }.isSuccess
        }
        try {
            val transport = TimeoutSocket(listener.localPort).apply { soTimeout = readTimeout }
            var admitted = false
            val result = runCatching {
                SecureConnection(keys("tablet"), fixture("$clientPeer.der")).connect(transport, remaining).use {
                    admitted = true
                    assertEquals("T846: handshake timeout must not leak into established streams", readTimeout, it.soTimeout)
                    it.getOutputStream().write(42); it.getOutputStream().flush()
                    assertEquals(43, it.getInputStream().read())
                }
            }
            if (admitted) assertTrue("T1522: TLS ignored configured setup budget: ${transport.timeouts}", setupTimeout in transport.timeouts)
            if (!sendPreface || !preface.contentEquals(fixture("protocol-v1.bin"))) {
                assertFalse("Invalid protocol must reject before returning any application stream", admitted)
            }
            if (result.isFailure) assertTrue(transport.isClosed)
            val accepted = server.get(5, TimeUnit.SECONDS)
            return result.isSuccess && accepted
        } finally { listener.close(); executor.shutdownNow(); assertTrue(executor.awaitTermination(5, TimeUnit.SECONDS)) }
    }
    private class TimeoutSocket(port: Int) : Socket(InetAddress.getLoopbackAddress(), port) {
        val timeouts = mutableListOf<Int>()
        override fun setSoTimeout(timeout: Int) { super.setSoTimeout(timeout); timeouts.add(timeout) }
    }
    @Test fun t1522TlsReadTimeoutUsesConfiguredSetupBudget() {
        assertTrue(trial(readTimeout = 321, setupTimeout = 25000))
    }
    @Test fun t846MutualTlsTransfersBytesOnlyForApprovedIdentities() {
        assertTrue(trial())
        assertFalse(trial(clientPeer = "stranger"))
        assertFalse(trial(serverPeer = "stranger"))
        assertFalse(trial(protocol = "other/1"))
        assertFalse(trial(protocol = null))
    }
    @Test fun t846HandshakePreservesCallerReadTimeout() {
        assertTrue(trial(readTimeout = 0))
        assertTrue(trial(readTimeout = 321))
    }
    @Test fun t846ProtocolPrefaceRejectsOtherVersionsAndMissingPeers() {
        assertFalse(trial(preface = ByteArray(10)))
        assertFalse(trial(preface = fixture("protocol-v1.bin").apply { this[lastIndex] = 2 }))
        assertFalse(trial(preface = byteArrayOf(1, 2)))
        assertFalse(trial(sendPreface = false))
    }
    @Test fun t846MalformedAndOversizedCertificatesReject() {
        for (length in listOf(0, 1, 4096, 4097, 8192)) {
            assertThrows(Exception::class.java) { SecureConnection(keys("tablet"), ByteArray(length)) }
        }
        val valid = fixture("host.der")
        assertThrows(IllegalArgumentException::class.java) { SecureConnection(keys("tablet"), valid + byteArrayOf(1)) }
        val random = java.util.Random(846)
        repeat(128) {
            val invalid = ByteArray(random.nextInt(128) + 1).apply { random.nextBytes(this); this[0] = 0 }
            assertThrows(Exception::class.java) { SecureConnection(keys("tablet"), invalid) }
        }
    }
    @Test @Config(sdk = [27]) fun t846Api27UsesAuthenticatedPrefaceWithoutPublicAlpn() {
        assertTrue(trial(protocol = null, tls12 = true))
        assertFalse(trial(clientPeer = "stranger", protocol = null))
        assertFalse(trial(serverPeer = "stranger", protocol = null))
        assertFalse(trial(preface = ByteArray(10)))
    }
    @Test fun t846InvalidTransportClosesOwnedSocket() {
        val socket = Socket()
        assertThrows(Exception::class.java) { SecureConnection(keys("tablet"), fixture("host.der")).connect(socket) { 3000 } }
        assertTrue(socket.isClosed)
    }
    @Test fun t1522InvalidTlsBudgetsCloseTransportBeforeAdmission() {
        for (budget in listOf(Int.MIN_VALUE, -1, 0, 60001, Int.MAX_VALUE)) {
            assertFalse(trial(setupTimeout = budget))
        }
        assertTrue(trial(setupTimeout = 60000))
    }
    @Test fun t1522ExpiredBudgetRejectsBeforeAndAfterPrefaceWithoutAdmittingStream() {
        for (expiryCall in 2..4) {
            var calls = 0
            assertFalse(trial(setupTimeout = 25000, remaining = {
                calls++
                if (calls == expiryCall) throw java.net.SocketTimeoutException("T1522 stage expired")
                25000
            }))
            assertEquals(expiryCall, calls)
        }
    }
}
