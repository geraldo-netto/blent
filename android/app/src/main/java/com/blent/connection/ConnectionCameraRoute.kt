package com.blent.connection

/** Private camera uplink metadata. The single IPv4 loopback listener originates
 * locally, never in remote management. Publication alone grants no camera access. */
internal data class LocalCameraRoute(
    val lease: ConnectionResourceLease,
    val credential: ConnectionManagement.Credential,
    val port: Int,
    val profile: ConnectionManagement.Camera,
) {
    init {
        ConnectionLocalLease.validate(lease, ConnectionManagement.Resource.CAMERA)
        require(port in 1..65535)
        ConnectionManagement.encode(ConnectionManagement.Offer(lease.key, credential, profile))
    }
}
