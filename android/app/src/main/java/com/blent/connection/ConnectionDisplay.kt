package com.blent.connection

/** Private local invitation, valid only while the exact lease remains current.
 * Addresses are fixed loopback; ports originate in local listeners, never wire
 * metadata. Reading this value conveys neither pairing nor media authority. */
internal data class LocalDisplayRoute(
    val lease: ConnectionResourceLease,
    val credential: ConnectionManagement.Credential,
    val controlPort: Int,
    val displayPort: Int,
) {
    init {
        ConnectionDisplayLease.validate(lease)
        require(controlPort in 1..65535 && displayPort in 1..65535 && controlPort != displayPort)
    }
}

/** Shape validation only; authority remains with the current runtime owner. */
internal object ConnectionDisplayLease {
    fun validate(lease: ConnectionResourceLease) {
        ConnectionLocalLease.validate(lease, ConnectionManagement.Resource.DISPLAY)
    }
}
