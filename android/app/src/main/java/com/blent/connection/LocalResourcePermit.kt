package com.blent.connection

/** Local opening and protocol admission are independent consent boundaries.
 * Implementations are prompt and include original runtime lifetime ownership. */
internal interface LocalResourcePermit {
    fun opening(lease: ConnectionResourceLease): Boolean
    fun admission(lease: ConnectionResourceLease): Boolean
}

/** Display requires no Android camera/audio permission. Other roles deny by
 * default and require their explicit, independently owned consent adapter. */
internal object DisplayLocalPermit : LocalResourcePermit {
    override fun opening(lease: ConnectionResourceLease) = lease.key.resource == ConnectionManagement.Resource.DISPLAY
    override fun admission(lease: ConnectionResourceLease) = opening(lease)
}

/** Boundary adapter reads a fresh UI/OS choice before each privileged transition.
 * A captured permission callback boolean cannot bypass the portable policy. */
internal class CameraLocalPermit(
    private val consent: ConnectionCameraConsent,
    private val choice: () -> CameraConsentChoice,
) : LocalResourcePermit {
    override fun opening(lease: ConnectionResourceLease) = consent.preparationAllowed(lease, choice())
    override fun admission(lease: ConnectionResourceLease) = consent.captureAllowed(lease, choice())
}
