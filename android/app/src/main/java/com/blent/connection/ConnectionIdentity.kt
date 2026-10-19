package com.blent.connection

import java.security.MessageDigest

/** Public identity only. Possession and peer approval are separate contracts. */
internal class ConnectionIdentity(certificate: ByteArray) {
    private val encoded: ByteArray
    init {
        require(certificate.size in 1..ConnectionHello.MAX_CERTIFICATE)
        encoded = certificate.copyOf()
    }
    fun certificate(): ByteArray = encoded.copyOf()
    /** Canonical fingerprint of the exact bounded public certificate bytes. */
    fun fingerprint(): ConnectionPeer {
        val digest = MessageDigest.getInstance("SHA-256").digest(encoded)
        return ConnectionPeer.parse(digest.joinToString("") { "%02x".format(it.toInt() and 255) })
    }
}

internal interface ConnectionIdentitySource {
    fun publicIdentity(): ConnectionIdentity
}
