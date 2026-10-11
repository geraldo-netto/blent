package com.blent.connection

/** Focused portable ownership boundary. Closing is logical; retirement requires
 * every retained native participant's actual completion, including consumers. */
internal interface LocalResourceOwnership {
    val offer: ConnectionSessionEvent.Offered
    fun open(now: Long, factory: (Long) -> LocalEndpoint)
    fun advance(now: Long)
    fun admit(now: Long)
    fun stop(now: Long)
    fun retired(): Boolean
    fun quarantined(): Boolean
}
internal interface LocalDisplayProjection { fun route(): LocalDisplayRoute? }

/** Permission metadata contains no endpoint or credential. Interpretation grants
 * neither OS permission nor original connection/worker lifetime authority. */
internal data class LocalCameraOffer(
    override val lease: ConnectionResourceLease,
    override val profile: ConnectionManagement.Camera,
    val permissionRequired: Boolean,
) : LocalMediaOffer<ConnectionManagement.Camera> {
    init {
        ConnectionLocalLease.validate(lease, ConnectionManagement.Resource.CAMERA)
        ConnectionManagement.encode(ConnectionManagement.Offer(lease.key,
            ConnectionManagement.Credential(ByteArray(32)), profile))
    }
}

/** Serialized exact-lease control. Publication records possible consumer exposure
 * BEFORE returning credentials. consumerRetired requires actual capture-worker
 * completion/no-start proof supplied by its retained native boundary adapter;
 * rejection, a timeout, socket EOF or sending cancellation is insufficient. */
internal interface LocalCameraProjection {
    fun offered(): LocalCameraOffer?
    fun camera(): LocalCameraRoute?
}
internal interface LocalCameraControl : LocalCameraProjection {
    fun pending(): CameraPermissionRequest?
    fun choose(lease: ConnectionResourceLease, choice: CameraConsentChoice, now: Long): Boolean
    fun permissionResult(prompt: CameraPermissionRequest, granted: Boolean, choice: CameraConsentChoice, now: Long): Boolean
    fun consumerRetired(lease: ConnectionResourceLease, now: Long): Boolean
}

/** Current pending/admitted resource authority at the original runtime boundary.
 * Pure and bounded: expiry is checked before a late UI callback can open native
 * work, even when the protocol timer has not yet processed its closing event. */
internal interface ConnectionResourceAuthority {
    fun owns(lease: ConnectionResourceLease, now: Long): Boolean
}
