package com.blent.connection

import java.security.*
import java.security.cert.Certificate
import java.security.cert.CertificateFactory
import java.security.interfaces.ECPrivateKey
import java.security.spec.PKCS8EncodedKeySpec

/** Test-only provider with committed synthetic material. Exercises the production
 * loader/provider boundary; never evidence of device AndroidKeyStore isolation. */
open class ConnectionFactoryKeyStoreFixture : KeyStoreSpi() {
    private class OpaqueKey(private val source: ECPrivateKey) : ECPrivateKey by source {
        override fun getEncoded(): ByteArray? = null
        override fun getFormat(): String? = null
    }
    private fun bytes(name: String) = javaClass.classLoader!!.getResourceAsStream("connection-tls/$name")!!.use { it.readBytes() }
    override fun engineGetEntry(alias: String?, protection: KeyStore.ProtectionParameter?): KeyStore.Entry {
        check(alias == AndroidConnectionIdentity.ALIAS)
        val key = KeyFactory.getInstance("EC").generatePrivate(PKCS8EncodedKeySpec(bytes("tablet.pk8"))) as ECPrivateKey
        val certificate = CertificateFactory.getInstance("X.509").generateCertificate(bytes("tablet.der").inputStream())
        return KeyStore.PrivateKeyEntry(OpaqueKey(key), arrayOf(certificate))
    }
    override fun engineContainsAlias(alias: String?) = alias == AndroidConnectionIdentity.ALIAS
    override fun engineLoad(stream: java.io.InputStream?, password: CharArray?) { check(stream == null && password == null) }
    override fun engineGetKey(alias: String?, password: CharArray?): Key = error("Unused")
    override fun engineGetCertificateChain(alias: String?): Array<Certificate> = error("Unused")
    override fun engineGetCertificate(alias: String?): Certificate = error("Unused")
    override fun engineGetCreationDate(alias: String?): java.util.Date = error("Unused")
    override fun engineSetKeyEntry(alias: String?, key: Key?, password: CharArray?, chain: Array<out Certificate>?) = error("No writes")
    override fun engineSetKeyEntry(alias: String?, key: ByteArray?, chain: Array<out Certificate>?) = error("No writes")
    override fun engineSetCertificateEntry(alias: String?, certificate: Certificate?) = error("No writes")
    override fun engineDeleteEntry(alias: String?) = error("No deletion")
    override fun engineAliases(): java.util.Enumeration<String> = java.util.Collections.enumeration(listOf(AndroidConnectionIdentity.ALIAS))
    override fun engineSize() = 1
    override fun engineIsKeyEntry(alias: String?) = alias == AndroidConnectionIdentity.ALIAS
    override fun engineIsCertificateEntry(alias: String?) = false
    override fun engineGetCertificateAlias(certificate: Certificate?): String = error("Unused")
    override fun engineStore(stream: java.io.OutputStream?, password: CharArray?) = error("No writes")
}
