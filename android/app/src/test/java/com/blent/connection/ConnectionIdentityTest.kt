package com.blent.connection

import java.io.File
import java.io.RandomAccessFile
import java.security.KeyFactory
import java.security.KeyStore
import java.security.PrivateKey
import java.security.cert.CertificateFactory
import java.security.cert.X509Certificate
import java.security.interfaces.ECPrivateKey
import java.security.spec.PKCS8EncodedKeySpec
import java.nio.channels.OverlappingFileLockException
import javax.security.auth.x500.X500Principal
import org.junit.Assert.*
import org.junit.Test

class ConnectionIdentityTest {
    private fun fixture(name: String): ByteArray = javaClass.classLoader!!
        .getResourceAsStream("connection-tls/$name")!!.use { it.readBytes() }
    private fun certificate(name: String = "tablet") = CertificateFactory.getInstance("X.509")
        .generateCertificate(fixture("$name.der").inputStream()) as X509Certificate
    private fun key(name: String = "tablet"): ECPrivateKey = KeyFactory.getInstance("EC")
        .generatePrivate(PKCS8EncodedKeySpec(fixture("$name.pk8"))) as ECPrivateKey
    /** Public test fixture wrapped like an opaque provider key; never used in production. */
    private class OpaqueKey(private val source: ECPrivateKey) : ECPrivateKey by source {
        override fun getEncoded(): ByteArray? = null
        override fun getFormat(): String? = null
    }
    private fun entry(name: String = "tablet", opaque: Boolean = true): KeyStore.PrivateKeyEntry {
        val privateKey: PrivateKey = if (opaque) OpaqueKey(key(name)) else key(name)
        return KeyStore.PrivateKeyEntry(privateKey, arrayOf(certificate()))
    }
    private class Store(var material: KeyStore.PrivateKeyEntry? = null) : IdentityStore {
        var exists = material != null
        var reads = 0
        override fun contains(alias: String): Boolean { assertEquals(AndroidConnectionIdentity.ALIAS, alias); return exists }
        override fun read(alias: String): KeyStore.PrivateKeyEntry { reads++; return material ?: error("Partial alias") }
    }
    private fun inDirectory(action: (File) -> Unit) {
        val directory = java.nio.file.Files.createTempDirectory("blent-identity-test").toFile()
        try { action(directory) } finally { directory.deleteRecursively() }
    }
    @Test fun t846PublicIdentityIsBoundedAndDefensivelyCopied() {
        val bytes = certificate().encoded
        val expected = bytes.copyOf()
        val identity = ConnectionIdentity(bytes)
        bytes.fill(0); identity.certificate().fill(1)
        assertArrayEquals(expected, identity.certificate())
        for (size in listOf(0, 4097, 8192)) assertThrows(IllegalArgumentException::class.java) { ConnectionIdentity(ByteArray(size)) }
        val random = java.util.Random(846)
        repeat(128) {
            val size = 1 + random.nextInt(4096)
            assertEquals(size, ConnectionIdentity(ByteArray(size)).certificate().size)
        }
    }
    @Test fun t846CreatesOnceAndReusesOpaqueIdentityWithoutTrustWrites() = inDirectory { directory ->
        val store = Store()
        var generated = 0
        val adapter = AndroidConnectionIdentity(directory, store, IdentityGenerator { alias ->
            assertEquals(AndroidConnectionIdentity.ALIAS, alias)
            generated++; store.material = entry(); store.exists = true
        })
        repeat(2) {
            val identity = adapter.load()
            assertArrayEquals(certificate().encoded, identity.publicIdentity().certificate())
            val managers = identity.keyManagers()
            assertEquals(1, managers.size)
            managers[0] = object : javax.net.ssl.KeyManager {}
            assertTrue(identity.keyManagers()[0] is IdentityKeyManager)
        }
        assertEquals(1, generated)
        assertEquals(2, store.reads)
    }
    @Test fun t846PersistedIdentityWithoutRawTlsSigningFailsBeforeUsbWithoutRotation() = inDirectory { directory ->
        val original = entry()
        val store = Store(original)
        val algorithms = mutableListOf<String>()
        val adapter = AndroidConnectionIdentity(directory, store, IdentityGenerator { fail("Existing identity must not rotate") }) { algorithm ->
            algorithms.add(algorithm)
            if (algorithm == "NONEwithECDSA") throw java.security.InvalidKeyException("T846 persisted key: INCOMPATIBLE_DIGEST")
            java.security.Signature.getInstance(algorithm)
        }
        val failure = assertThrows("T846: SHA256 proof alone misses Conscrypt's raw ECDSA requirement", java.security.InvalidKeyException::class.java) { adapter.load() }
        assertTrue(failure.message!!.contains("existing alias preserved"))
        assertTrue(failure.cause!!.message!!.contains("INCOMPATIBLE_DIGEST"))
        assertTrue(algorithms.contains("SHA256withECDSA")); assertTrue(algorithms.contains("NONEwithECDSA"))
        assertSame(original, store.material); assertTrue(store.exists); assertEquals(1, store.reads)
    }
    @Test fun t846IdentityProvesBothCertificateAndNativeTlsSigning() = inDirectory { directory ->
        val algorithms = mutableListOf<String>()
        val adapter = AndroidConnectionIdentity(directory, Store(entry()), IdentityGenerator { fail("Unexpected generation") }) { algorithm ->
            algorithms.add(algorithm); java.security.Signature.getInstance(algorithm)
        }
        assertArrayEquals(certificate().encoded, adapter.load().publicIdentity().certificate())
        assertTrue(algorithms.contains("SHA256withECDSA")); assertTrue(algorithms.contains("NONEwithECDSA"))
    }
    @Test fun t846InvalidExistingKeysNeverRotateOrDelete() = inDirectory { directory ->
        val bad = listOf(null, entry(opaque = false), entry(name = "stranger"))
        for (material in bad) {
            val store = Store(material).apply { exists = true }
            val adapter = AndroidConnectionIdentity(directory, store, IdentityGenerator { fail("Existing alias must never rotate") })
            assertThrows(Exception::class.java) { adapter.load() }
            assertTrue(store.exists)
        }
        // Failed validation released the lock, allowing a later valid load.
        val good = AndroidConnectionIdentity(directory, Store(entry()), IdentityGenerator { fail("Unexpected generation") })
        assertArrayEquals(certificate().encoded, good.load().publicIdentity().certificate())
    }
    @Test fun t846InvalidCertificatesAndCurvesFailClosed() = inDirectory { directory ->
        val otherCurve = java.security.KeyPairGenerator.getInstance("EC").apply {
            initialize(java.security.spec.ECGenParameterSpec("secp384r1"))
        }.generateKeyPair().public
        val corrupt = listOf(
            IdentityCertificateFixture(certificate()).apply { expired = true },
            IdentityCertificateFixture(certificate()).apply { validSignature = false },
            IdentityCertificateFixture(certificate()).apply { issuer = X500Principal("CN=other") },
            IdentityCertificateFixture(certificate()).apply { bytes = ByteArray(4097) },
            IdentityCertificateFixture(certificate()).apply { bytes = ByteArray(0) },
            IdentityCertificateFixture(certificate()).apply { key = otherCurve },
        )
        for (certificate in corrupt) {
            val entry = KeyStore.PrivateKeyEntry(OpaqueKey(key()), arrayOf(certificate))
            val adapter = AndroidConnectionIdentity(directory, Store(entry), IdentityGenerator { fail("Never rotate") })
            assertThrows(Exception::class.java) { adapter.load() }
        }
        val chain = KeyStore.PrivateKeyEntry(OpaqueKey(key()), arrayOf(certificate(), certificate()))
        assertThrows(IllegalStateException::class.java) {
            AndroidConnectionIdentity(directory, Store(chain), IdentityGenerator { fail("Never rotate") }).load()
        }
    }
    @Test fun t846GenerationFailurePreservesPartialAliasAndReleasesLock() = inDirectory { directory ->
        val store = Store()
        var generated = 0
        val adapter = AndroidConnectionIdentity(directory, store, IdentityGenerator {
            generated++; store.exists = true; error("Native generation interrupted")
        })
        repeat(2) { assertThrows(IllegalStateException::class.java) { adapter.load() } }
        assertEquals(1, generated)
        assertTrue(store.exists)
    }
    @Test fun t846ConcurrentOwnersRejectBeforeTouchingKeyStore() = inDirectory { directory ->
        val store = Store(entry())
        val adapter = AndroidConnectionIdentity(directory, store, IdentityGenerator { fail("Unexpected generation") })
        RandomAccessFile(File(directory, "accessory-identity.lock"), "rw").use { file ->
            file.channel.lock().use {
                assertThrows(OverlappingFileLockException::class.java) { adapter.load() }
                assertEquals(0, store.reads)
            }
        }
        assertNotNull(adapter.load())
    }
    @Test fun t846KeyManagerExposesOnlySelectedClientAlias() {
        val cert = certificate()
        val privateKey = OpaqueKey(key())
        val manager = IdentityKeyManager("owned", privateKey, cert)
        assertArrayEquals(arrayOf("owned"), manager.getClientAliases("EC", null))
        assertNull(manager.getClientAliases("RSA", null))
        assertNull(manager.getClientAliases(null, null))
        assertNull(manager.getClientAliases("EC", arrayOf(X500Principal("CN=untrusted issuer"))))
        assertArrayEquals(arrayOf("owned"), manager.getClientAliases("EC", arrayOf(cert.issuerX500Principal)))
        assertEquals("owned", manager.chooseClientAlias(arrayOf("RSA", "EC"), emptyArray(), null))
        assertNull(manager.chooseClientAlias(arrayOf("RSA"), null, null))
        assertNull(manager.chooseClientAlias(null, null, null))
        assertEquals("owned", manager.chooseEngineClientAlias(arrayOf("EC"), null, null))
        assertNull(manager.chooseEngineClientAlias(emptyArray(), null, null))
        assertSame(privateKey, manager.getPrivateKey("owned"))
        assertNull(manager.getPrivateKey("other"))
        assertNull(manager.getCertificateChain("other"))
        val chain = manager.getCertificateChain("owned")!!
        chain[0] = certificate("stranger")
        assertEquals(cert, manager.getCertificateChain("owned")!![0])
        assertNull(manager.getServerAliases("EC", null))
        assertNull(manager.chooseServerAlias("EC", null, null))
        assertNull(manager.chooseEngineServerAlias("EC", null, null))
    }
}
