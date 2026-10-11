package com.blent.connection

/** Private camera uplink metadata. The single IPv4 loopback listener originates
 * locally, never in remote management. Publication alone grants no camera access. */
internal data class LocalCameraRoute(
    override val lease: ConnectionResourceLease,
    override val credential: ConnectionManagement.Credential,
    override val port: Int,
    override val profile: ConnectionManagement.Camera,
) : LocalMediaRoute<ConnectionManagement.Camera> {
    init {
        ConnectionLocalLease.validate(lease, ConnectionManagement.Resource.CAMERA)
        require(port in 1..65535)
        ConnectionManagement.encode(ConnectionManagement.Offer(lease.key, credential, profile))
    }
}
