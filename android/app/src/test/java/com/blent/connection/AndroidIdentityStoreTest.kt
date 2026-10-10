package com.blent.connection

import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import java.security.*
import java.security.cert.Certificate
import java.security.cert.CertificateFactory
import java.security.spec.AlgorithmParameterSpec
import java.security.spec.ECGenParameterSpec
import java.security.spec.PKCS8EncodedKeySpec
import java.util.Date
import java.util.Collections
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [34])
class AndroidIdentityStoreTest {
    private fun fixture(name: String): ByteArray = javaClass.classLoader!!
        .getResourceAsStream("connection-tls/$name")!!.use { it.readBytes() }
    private fun entry(): KeyStore.PrivateKeyEntry {
        val cert = CertificateFactory.getInstance("X.509").generateCertificate(fixture("tablet.der").inputStream())
        val key = KeyFactory.getInstance("EC").generatePrivate(PKCS8EncodedKeySpec(fixture("tablet.pk8")))
        return KeyStore.PrivateKeyEntry(key, arrayOf(cert))
    }
    private class StoreSpi(var entry: KeyStore.Entry?) : KeyStoreSpi() {
        override fun engineGetEntry(alias: String?, protection: KeyStore.ProtectionParameter?): KeyStore.Entry? = entry
        override fun engineContainsAlias(alias: String?): Boolean = entry != null
        override fun engineLoad(stream: java.io.InputStream?, password: CharArray?) { assertNull(stream); assertNull(password) }
        override fun engineGetKey(alias: String?, password: CharArray?): Key? = error("Unused")
        override fun engineGetCertificateChain(alias: String?): Array<Certificate>? = error("Unused")
        override fun engineGetCertificate(alias: String?): Certificate? = error("Unused")
        override fun engineGetCreationDate(alias: String?): Date = error("Unused")
        override fun engineSetKeyEntry(alias: String?, key: Key?, password: CharArray?, chain: Array<out Certificate>?) { error("Unused") }
        override fun engineSetKeyEntry(alias: String?, key: ByteArray?, chain: Array<out Certificate>?) { error("Unused") }
        override fun engineSetCertificateEntry(alias: String?, certificate: Certificate?) { error("Unused") }
        override fun engineDeleteEntry(alias: String?) { error("Never delete identities") }
        override fun engineAliases(): java.util.Enumeration<String> = Collections.emptyEnumeration()
        override fun engineSize(): Int = error("Unused")
        override fun engineIsKeyEntry(alias: String?): Boolean = error("Unused")
        override fun engineIsCertificateEntry(alias: String?): Boolean = error("Unused")
        override fun engineGetCertificateAlias(certificate: Certificate?): String? = error("Unused")
        override fun engineStore(stream: java.io.OutputStream?, password: CharArray?) { error("Unused") }
    }
    private class TestStore(spi: StoreSpi) : KeyStore(spi, object : Provider("Test", 1.0, "Test only") {}, "Test")
    private class Generator(private val result: KeyPair) : KeyPairGenerator("EC") {
        var parameters: KeyGenParameterSpec? = null
        override fun initialize(spec: AlgorithmParameterSpec?, random: SecureRandom?) { parameters = spec as KeyGenParameterSpec }
        override fun generateKeyPair(): KeyPair = result
    }
    @Test fun t846StoreUsesOnlyAndroidProviderAndRequiresPrivateEntry() {
        val expected = entry()
        val spi = StoreSpi(expected)
        val store = AndroidIdentityStore.open { name -> assertEquals("AndroidKeyStore", name); TestStore(spi) }
        assertTrue(store.contains("owned"))
        assertSame(expected, store.read("owned"))
        spi.entry = KeyStore.TrustedCertificateEntry(expected.certificate)
        assertThrows(IllegalStateException::class.java) { store.read("owned") }
        spi.entry = null
        assertFalse(store.contains("missing"))
        assertThrows(IllegalStateException::class.java) { store.read("missing") }
        assertThrows(KeyStoreException::class.java) { AndroidIdentityStore.open { throw KeyStoreException("Unavailable") } }
    }
    @Test fun t846GenerationUsesP256SigningWithoutExportOrUserUnlock() {
        val material = entry()
        val generator = Generator(KeyPair(material.certificate.publicKey, material.privateKey))
        AndroidIdentityGenerator { algorithm, provider ->
            assertEquals("EC", algorithm); assertEquals("AndroidKeyStore", provider); generator
        }.generate("blent.test.identity")
        val spec = generator.parameters!!
        assertEquals("blent.test.identity", spec.keystoreAlias)
        assertEquals(KeyProperties.PURPOSE_SIGN, spec.purposes)
        assertArrayEquals(arrayOf(KeyProperties.DIGEST_NONE, KeyProperties.DIGEST_SHA256), spec.digests)
        assertEquals("secp256r1", (spec.algorithmParameterSpec as ECGenParameterSpec).name)
        assertFalse(spec.isUserAuthenticationRequired)
        assertEquals("CN=Blent Android client", spec.certificateSubject.name)
        assertEquals(java.math.BigInteger.ONE, spec.certificateSerialNumber)
        assertEquals(0L, spec.certificateNotBefore.time)
        assertEquals(253402300799000L, spec.certificateNotAfter.time)
    }
    @Test @Config(sdk = [27]) fun t846Api27UsesAvailableKeyGenerationContract() {
        t846GenerationUsesP256SigningWithoutExportOrUserUnlock()
    }
    @Test fun t846UnavailableNativeProviderDoesNotUseExportableSoftwareFallback() {
        assertThrows(KeyStoreException::class.java) { AndroidIdentityStore.open() }
        val generator = AndroidIdentityGenerator()
        assertThrows(NoSuchProviderException::class.java) { generator.generate("owned") }
    }
    @Test fun t846GenerationProviderFailurePropagatesWithoutFallback() {
        assertThrows(NoSuchProviderException::class.java) {
            AndroidIdentityGenerator { _, _ -> throw NoSuchProviderException("Unavailable") }.generate("owned")
        }
    }
}
