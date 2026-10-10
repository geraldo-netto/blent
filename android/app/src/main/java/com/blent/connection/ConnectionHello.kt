package com.blent.connection

/** Untrusted public identity candidate only. Approval and TLS proof are separate. */
internal object ConnectionHello {
    const val HEADER_BYTES = 8
    const val MAX_CERTIFICATE = 4096
    fun encode(host: Boolean, length: Int): ByteArray {
        require(length in 1..MAX_CERTIFICATE)
        return byteArrayOf(66, 76, 80, 65, 1, if (host) 1 else 0, (length ushr 8).toByte(), length.toByte())
    }
    fun decode(bytes: ByteArray, expectHost: Boolean): Int {
        require(bytes.size == HEADER_BYTES)
        val length = ((bytes[6].toInt() and 255) shl 8) or (bytes[7].toInt() and 255)
        require(bytes.contentEquals(encode(expectHost, length)))
        return length
    }
}
