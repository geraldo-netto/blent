package com.blent.connection

import java.io.File
import java.net.Socket
import java.nio.file.Files
import java.security.KeyFactory
import java.security.KeyStore
import java.security.cert.CertificateFactory
import java.security.spec.PKCS8EncodedKeySpec
import java.util.concurrent.TimeUnit
import javax.net.ssl.KeyManagerFactory
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config

/** T846: real maintained Android adapter against the maintained Rust accept path.
 * Runs only in tlsInteropTest, required by scripts/tests/test_secure_connection_interop.py. */
@RunWith(RobolectricTestRunner::class)
@Config(sdk = [34])
class SecureConnectionInteropTest {
    private fun fixture(name: String) = javaClass.classLoader!!
        .getResourceAsStream("connection-tls/$name")!!.use { it.readBytes() }

    private fun client(peer: String): SecureConnection {
        val certificate = CertificateFactory.getInstance("X.509").generateCertificate(fixture("tablet.der").inputStream())
        val privateKey = KeyFactory.getInstance("EC").generatePrivate(PKCS8EncodedKeySpec(fixture("tablet.pk8")))
        val store = KeyStore.getInstance(KeyStore.getDefaultType()).apply {
            load(null, null)
            setKeyEntry("tablet", privateKey, charArrayOf(), arrayOf(certificate))
        }
        val keys = KeyManagerFactory.getInstance(KeyManagerFactory.getDefaultAlgorithm()).apply { init(store, charArrayOf()) }
        return SecureConnection(keys.keyManagers, fixture("$peer.der"))
    }

    private fun port(process: Process, ready: File): Int {
        val deadline = System.nanoTime() + TimeUnit.SECONDS.toNanos(5)
        while (System.nanoTime() < deadline) {
            check(process.isAlive) { "Rust fixture exited before readiness" }
            if (ready.isFile && ready.length() in 2..6) {
                val value = ready.readText().trim().toIntOrNull()
                if (value != null && value in 1..65535) return value
            }
            Thread.sleep(10)
        }
        error("Rust fixture readiness deadline")
    }

    private fun trial(peer: String = "host", mode: String = "accept"): Boolean {
        val directory = Files.createTempDirectory("blent-t846-interop-").toFile()
        try {
            listOf("host.der", "host.pk8", "tablet.der").forEach { File(directory, it).writeBytes(fixture(it)) }
            return runPeer(directory, peer, mode)
        } finally { directory.deleteRecursively() }
    }

    private fun runPeer(directory: File, peer: String, mode: String): Boolean {
        val executable = checkNotNull(System.getProperty("blent.tls.fixture")) { "Missing Rust TLS fixture" }
        val ready = File(directory, "ready")
        val log = File(directory, "fixture.log")
        val process = ProcessBuilder(executable, directory.absolutePath, ready.absolutePath, mode)
            .redirectErrorStream(true).redirectOutput(log).start()
        try {
            val socket = Socket("127.0.0.1", port(process, ready)).apply { soTimeout = 3000 }
            var admitted = false
            val result = runCatching {
                client(peer).connect(socket).use { secure ->
                    admitted = true
                    secure.getOutputStream().write("fixture".toByteArray()); secure.getOutputStream().flush()
                    val reply = ByteArray(8)
                    java.io.DataInputStream(secure.getInputStream()).readFully(reply)
                    assertArrayEquals("accepted".toByteArray(), reply)
                }
            }
            if (peer != "host" || mode == "wrong-preface") assertFalse("Unapproved stream escaped authentication gate", admitted)
            if (result.isFailure) assertTrue(socket.isClosed)
            assertTrue("Rust fixture failed to retire: ${log.readText()}", process.waitFor(5, TimeUnit.SECONDS))
            if (mode == "wrong-preface") assertEquals("Fault injector must complete TLS and write bad preface", 0, process.exitValue())
            return result.isSuccess && process.exitValue() == 0
        } finally {
            if (process.isAlive) process.destroyForcibly()
            check(process.waitFor(2, TimeUnit.SECONDS)) { "Rust fixture still owns resources" }
        }
    }

    @Test fun t846Api34RustAndroidMutualTlsAndRejection() {
        assertTrue(trial())
        assertFalse(trial(peer = "stranger"))
        assertFalse(trial(mode = "wrong-preface"))
    }

    @Test @Config(sdk = [27]) fun t846Api27RustAndroidPrefaceWithoutAlpn() {
        assertTrue(trial())
        assertFalse(trial(peer = "stranger"))
        assertFalse(trial(mode = "wrong-preface"))
    }
}
