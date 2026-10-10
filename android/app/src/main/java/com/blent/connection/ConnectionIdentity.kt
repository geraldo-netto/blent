package com.blent.connection

/** Public identity only. Possession and peer approval are separate contracts. */
internal class ConnectionIdentity(certificate: ByteArray) {
    private val encoded: ByteArray
    init {
        require(certificate.size in 1..ConnectionHello.MAX_CERTIFICATE)
        encoded = certificate.copyOf()
    }
    fun certificate(): ByteArray = encoded.copyOf()
}

internal interface ConnectionIdentitySource {
    fun publicIdentity(): ConnectionIdentity
}
