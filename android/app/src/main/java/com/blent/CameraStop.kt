package com.blent

/** A validated original endpoint key; neither permission nor native death proof. */
internal class CameraStop private constructor(private val credential: String, private val port: Int) {
    fun matches(endpoint: CameraEndpoint): Boolean = port == endpoint.port && credential == endpoint.token

    companion object {
        fun create(credential: String, port: Int): CameraStop? {
            if (!CameraCredential.valid(credential) || port !in 1..65535) return null
            return CameraStop(credential, port)
        }
    }
}

internal object CameraCredential {
    fun valid(value: String): Boolean = value.length == 64 && value.all { it in '0'..'9' || it in 'a'..'f' }
}
