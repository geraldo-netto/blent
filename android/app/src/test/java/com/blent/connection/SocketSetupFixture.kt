package com.blent.connection

import java.io.ByteArrayOutputStream
import java.io.InputStream
import java.net.InetAddress
import java.net.ServerSocket
import java.net.Socket
import java.security.KeyFactory
import java.security.KeyStore
import java.security.cert.CertificateFactory
import java.security.cert.X509Certificate
import java.security.spec.PKCS8EncodedKeySpec
import javax.net.ssl.*

internal object SocketSetupFixture {
    fun bytes(name: String): ByteArray = javaClass.classLoader!!.getResourceAsStream("connection-tls/$name")!!.use { it.readBytes() }
    private fun cert(name: String) = CertificateFactory.getInstance("X.509").generateCertificate(bytes("$name.der").inputStream()) as X509Certificate
    fun identity(): ClientTlsIdentity {
        val certificate = cert("tablet")
        val key = KeyFactory.getInstance("EC").generatePrivate(PKCS8EncodedKeySpec(bytes("tablet.pk8")))
        return ClientTlsIdentity(ConnectionIdentity(certificate.encoded), IdentityKeyManager("tablet", key, certificate))
    }
    fun hello(): ByteArray = bytes("host.der").let { ConnectionHello.encode(true, it.size) + it }
    fun listener() = ServerSocket(0, 1, InetAddress.getLoopbackAddress()).apply { soTimeout = 3000 }
    private fun context(): SSLContext {
        val store = KeyStore.getInstance(KeyStore.getDefaultType()).apply {
            load(null, null)
            val key = KeyFactory.getInstance("EC").generatePrivate(PKCS8EncodedKeySpec(bytes("host.pk8")))
            setKeyEntry("host", key, charArrayOf(), arrayOf(cert("host")))
        }
        val keys = KeyManagerFactory.getInstance(KeyManagerFactory.getDefaultAlgorithm()).apply { init(store, charArrayOf()) }
        val peers = KeyStore.getInstance(KeyStore.getDefaultType()).apply { load(null, null); setCertificateEntry("tablet", cert("tablet")) }
        val trusts = TrustManagerFactory.getInstance(TrustManagerFactory.getDefaultAlgorithm()).apply { init(peers) }
        return SSLContext.getInstance("TLS").apply { init(keys.keyManagers, trusts.trustManagers, null) }
    }
    private fun publicExchange(raw: Socket) {
        raw.soTimeout = 3000
        raw.outputStream.write(hello()); raw.outputStream.flush()
        val input = java.io.DataInputStream(raw.inputStream)
        val header = ByteArray(ConnectionHello.HEADER_BYTES).also(input::readFully)
        val certificate = ByteArray(ConnectionHello.decode(header, false)).also(input::readFully)
        check(certificate.contentEquals(bytes("tablet.der")))
        raw.outputStream.write(ConnectionReady.encode(true)); raw.outputStream.flush()
        ConnectionReady.decode(ByteArray(ConnectionReady.BYTES).also(input::readFully), false)
    }
    fun serve(listener: ServerSocket) {
        listener.accept().use { raw ->
            publicExchange(raw)
            val tls = context().socketFactory.createSocket(raw, "blent.local", raw.port, true) as SSLSocket
            tls.useClientMode = false; tls.needClientAuth = true; tls.soTimeout = 3000
            tls.sslParameters = tls.sslParameters.apply { applicationProtocols = arrayOf("blent-usb/1") }
            tls.use { exchange(it) }
        }
    }
    private fun exchange(tls: SSLSocket) {
        tls.startHandshake()
        val input = java.io.DataInputStream(tls.inputStream)
        val preface = bytes("protocol-v1.bin")
        tls.outputStream.write(preface); tls.outputStream.flush()
        check(ByteArray(preface.size).also(input::readFully).contentEquals(preface))
        tls.outputStream.write(ConnectionWindow.encode(true, 65536)); tls.outputStream.flush()
        check(ConnectionWindow.decode(ByteArray(ConnectionWindow.BYTES).also(input::readFully), false) == 4096)
        check(input.readUnsignedByte() == 42)
        tls.outputStream.write(43); tls.outputStream.flush()
    }
}

internal class SetupMemorySocket(bytes: ByteArray = SocketSetupFixture.hello()) : Socket() {
    val output = ByteArrayOutputStream()
    private val source = bytes.inputStream()
    var afterRead: () -> Unit = {}
    var maximumRead = Int.MAX_VALUE
    var readResult: Int? = null
    var closeFailure = false
    var closed = false
    private val input = object : InputStream() {
        override fun read(): Int = source.read()
        override fun read(bytes: ByteArray, offset: Int, length: Int): Int {
            val result = readResult ?: source.read(bytes, offset, minOf(length, maximumRead))
            afterRead(); return result
        }
    }
    override fun getInputStream(): InputStream = input
    override fun getOutputStream() = output
    override fun setSoTimeout(timeout: Int) { require(timeout >= 0) }
    override fun close() { closed = true; if (closeFailure) error("native close failed") }
}
