package com.blent.connection

import java.security.PublicKey
import java.security.cert.CertificateExpiredException
import java.security.cert.X509Certificate
import java.util.Date
import javax.security.auth.x500.X500Principal

/** Fault injection around committed public fixture; no production cryptography. */
internal class IdentityCertificateFixture(private val certificate: X509Certificate) : X509Certificate() {
    var bytes = certificate.encoded
    var key = certificate.publicKey
    var issuer = certificate.issuerX500Principal
    var expired = false
    var validSignature = true
    override fun getEncoded(): ByteArray = bytes
    override fun getPublicKey(): PublicKey = key
    override fun getIssuerX500Principal(): X500Principal = issuer
    override fun getSubjectX500Principal(): X500Principal = certificate.subjectX500Principal
    override fun checkValidity() { if (expired) throw CertificateExpiredException("Injected expiry") }
    override fun checkValidity(date: Date?) = checkValidity()
    override fun verify(key: PublicKey?) { check(validSignature) { "Injected invalid signature" } }
    override fun verify(key: PublicKey?, provider: String?) = verify(key)
    override fun toString(): String = "IdentityCertificateFixture"
    override fun getVersion(): Int = certificate.version
    override fun getSerialNumber(): java.math.BigInteger = certificate.serialNumber
    override fun getIssuerDN(): java.security.Principal = issuer
    override fun getSubjectDN(): java.security.Principal = certificate.subjectX500Principal
    override fun getNotBefore(): Date = certificate.notBefore
    override fun getNotAfter(): Date = certificate.notAfter
    override fun getTBSCertificate(): ByteArray = certificate.tbsCertificate
    override fun getSignature(): ByteArray = certificate.signature
    override fun getSigAlgName(): String = certificate.sigAlgName
    override fun getSigAlgOID(): String = certificate.sigAlgOID
    override fun getSigAlgParams(): ByteArray? = certificate.sigAlgParams
    override fun getIssuerUniqueID(): BooleanArray? = certificate.issuerUniqueID
    override fun getSubjectUniqueID(): BooleanArray? = certificate.subjectUniqueID
    override fun getKeyUsage(): BooleanArray? = certificate.keyUsage
    override fun getBasicConstraints(): Int = certificate.basicConstraints
    override fun getCriticalExtensionOIDs(): MutableSet<String>? = certificate.criticalExtensionOIDs
    override fun getNonCriticalExtensionOIDs(): MutableSet<String>? = certificate.nonCriticalExtensionOIDs
    override fun getExtensionValue(oid: String?): ByteArray? = certificate.getExtensionValue(oid)
    override fun hasUnsupportedCriticalExtension(): Boolean = certificate.hasUnsupportedCriticalExtension()
}
