package com.blent.connection

import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import java.math.BigInteger
import java.security.KeyPairGenerator
import java.security.KeyStore
import java.security.spec.ECGenParameterSpec
import java.util.Date
import javax.security.auth.x500.X500Principal

/** Injected native boundary. Never delete or replace an existing alias implicitly. */
internal interface IdentityStore {
    fun contains(alias: String): Boolean
    fun read(alias: String): KeyStore.PrivateKeyEntry
}
internal fun interface IdentityGenerator { fun generate(alias: String) }

internal class AndroidIdentityStore(private val store: KeyStore) : IdentityStore {
    companion object {
        fun open(factory: (String) -> KeyStore = KeyStore::getInstance): AndroidIdentityStore {
            val store = factory("AndroidKeyStore")
            store.load(null, null)
            return AndroidIdentityStore(store)
        }
    }
    override fun contains(alias: String): Boolean = store.containsAlias(alias)
    override fun read(alias: String): KeyStore.PrivateKeyEntry =
        store.getEntry(alias, null) as? KeyStore.PrivateKeyEntry ?: error("Missing client private key")
}

/** Stock provider owns generation and key persistence. No exported private key.
 * Conscrypt signs its prehashed TLS transcript with NONEwithECDSA; SHA256 alone
 * permits the certificate proof but rejects native TLS with INCOMPATIBLE_DIGEST. */
internal class AndroidIdentityGenerator(
    private val factory: (String, String) -> KeyPairGenerator = KeyPairGenerator::getInstance,
) : IdentityGenerator {
    override fun generate(alias: String) {
        val parameters = KeyGenParameterSpec.Builder(alias, KeyProperties.PURPOSE_SIGN)
            .setAlgorithmParameterSpec(ECGenParameterSpec("secp256r1"))
            .setDigests(KeyProperties.DIGEST_NONE, KeyProperties.DIGEST_SHA256)
            .setCertificateSubject(X500Principal("CN=Blent Android client"))
            .setCertificateSerialNumber(BigInteger.ONE)
            .setCertificateNotBefore(Date(0))
            .setCertificateNotAfter(Date(253402300799000L))
            .setUserAuthenticationRequired(false)
            .build()
        factory("EC", "AndroidKeyStore").apply { initialize(parameters); generateKeyPair() }
    }
}
