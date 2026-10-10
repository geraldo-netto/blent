package com.blent.connection

import java.io.File
import java.io.RandomAccessFile
import java.security.AlgorithmParameters
import java.security.KeyStore
import java.security.GeneralSecurityException
import java.security.InvalidKeyException
import java.security.SecureRandom
import java.security.Signature
import java.security.cert.X509Certificate
import java.security.interfaces.ECPublicKey
import java.security.spec.ECGenParameterSpec
import java.security.spec.ECParameterSpec
import javax.net.ssl.KeyManager

/** Loaded TLS boundary. Domain callers receive public bytes, never private keys. */
internal class ClientTlsIdentity internal constructor(
    private val identity: ConnectionIdentity,
    private val manager: IdentityKeyManager,
) : ConnectionIdentitySource {
    override fun publicIdentity(): ConnectionIdentity = identity
    fun keyManagers(): Array<KeyManager> = arrayOf(manager)
}

/** Run only inside the supervised connection worker. Native provider calls can
 * block; a lock failure is retryable only after the other owner actually retires.
 * The directory must be application-private and shared by its worker processes.
 * Failure preserves existing/partially generated keys for explicit repair; no
 * automatic rotation, deletion, or persisted peer approval occurs here. */
internal class AndroidConnectionIdentity(
    private val directory: File,
    private val store: IdentityStore,
    private val generator: IdentityGenerator,
    private val signatures: (String) -> Signature = Signature::getInstance,
) {
    companion object { const val ALIAS = "blent.accessory.client.v1" }

    fun load(): ClientTlsIdentity = RandomAccessFile(File(directory, "accessory-identity.lock"), "rw").use { file ->
        val lock = file.channel.tryLock() ?: error("Client identity is owned by another worker")
        lock.use {
            if (!store.contains(ALIAS)) generator.generate(ALIAS)
            validated(store.read(ALIAS))
        }
    }

    private fun validated(entry: KeyStore.PrivateKeyEntry): ClientTlsIdentity {
        check(entry.certificateChain.size == 1) { "Unexpected client certificate chain" }
        val certificate = entry.certificate as? X509Certificate ?: error("Client identity must be X.509")
        val identity = ConnectionIdentity(certificate.encoded)
        certificate.checkValidity()
        check(certificate.subjectX500Principal == certificate.issuerX500Principal)
        certificate.verify(certificate.publicKey)
        validateCurve(certificate)
        check(entry.privateKey.algorithm == "EC" && entry.privateKey.encoded == null) { "Client key must be non-exportable EC" }
        proveKey(entry, "SHA256withECDSA")
        proveNativeTlsKey(entry)
        return ClientTlsIdentity(identity, IdentityKeyManager(ALIAS, entry.privateKey, certificate))
    }

    private fun validateCurve(certificate: X509Certificate) {
        val actual = (certificate.publicKey as? ECPublicKey ?: error("Client key must be EC")).params
        val expected = AlgorithmParameters.getInstance("EC").apply { init(ECGenParameterSpec("secp256r1")) }
            .getParameterSpec(ECParameterSpec::class.java)
        check(actual.curve == expected.curve && actual.generator == expected.generator)
        check(actual.order == expected.order && actual.cofactor == expected.cofactor) { "Client key must be P-256" }
    }

    private fun proveNativeTlsKey(entry: KeyStore.PrivateKeyEntry) {
        try { proveKey(entry, "NONEwithECDSA") }
        catch (failure: GeneralSecurityException) {
            throw InvalidKeyException("Client identity cannot sign native TLS digests; existing alias preserved for explicit repair", failure)
        }
    }

    private fun proveKey(entry: KeyStore.PrivateKeyEntry, algorithm: String) {
        val challenge = ByteArray(32).also { SecureRandom().nextBytes(it) }
        val signature = signatures(algorithm).apply {
            initSign(entry.privateKey); update(challenge)
        }.sign()
        val verifier = signatures(algorithm).apply {
            initVerify(entry.certificate); update(challenge)
        }
        check(verifier.verify(signature)) { "Client certificate does not match private key" }
    }
}
