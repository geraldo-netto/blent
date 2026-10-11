package com.blent.connection

/** Authenticated ownership is checked for pending offers as well as admission.
 * Proof includes the original runtime/worker lifetime even if counters repeat.
 * Native composition supplies these proofs; a parsed lease supplies neither. */
internal interface CameraConsentAuthority : ConnectionMediaConsentAuthority

/** Snapshot of local intent and current OS permission, supplied by the UI adapter.
 * A granted callback alone is insufficient proof that permission still exists. */
internal data class CameraConsentChoice(
    val foreground: Boolean,
    val permission: Boolean,
    val enabled: Boolean = true,
    val backgroundAllowed: Boolean = false,
)

/** Opaque prompt identity. Even an equal lease in another owner cannot reuse it.
 * IPC adapters must resolve replies through their retained exact worker/prompt. */
internal class CameraPermissionRequest(val lease: ConnectionResourceLease)

internal sealed interface CameraConsentAction {
    data class Permission(val request: CameraPermissionRequest) : CameraConsentAction
    data class Prepare(val offer: ConnectionSessionEvent.Offered) : CameraConsentAction
    data class Reject(val lease: ConnectionResourceLease, val outcome: ConnectionManagement.Outcome) : CameraConsentAction
    data class Cancel(val lease: ConnectionResourceLease, val reason: ConnectionManagement.Reason) : CameraConsentAction
}

/** Typed camera boundary around the shared original media consent policy. The
 * camera profile/action contract remains independent of the generic implementation. */
internal class ConnectionCameraConsent(
    val offer: ConnectionSessionEvent.Offered,
    authority: CameraConsentAuthority,
) {
    private val consent = ConnectionMediaConsent(offer.lease, validate(offer).background, true, authority, ::CameraPermissionRequest)
    fun pending(): CameraPermissionRequest? = consent.pending()
    fun closing(): Boolean = consent.closing()
    fun update(choice: CameraConsentChoice): CameraConsentAction? = action(consent.update(context(choice)))
    fun permissionResult(prompt: CameraPermissionRequest, granted: Boolean, choice: CameraConsentChoice): CameraConsentAction? =
        action(consent.permissionResult(prompt, granted, context(choice)))
    fun preparationAllowed(lease: ConnectionResourceLease, choice: CameraConsentChoice): Boolean =
        consent.preparationAllowed(lease, context(choice))
    fun captureAllowed(lease: ConnectionResourceLease, choice: CameraConsentChoice): Boolean =
        consent.captureAllowed(lease, context(choice))
    fun stop(): CameraConsentAction? = action(consent.stop())
    fun connectionLost(): CameraConsentAction? = action(consent.connectionLost())
    private fun context(choice: CameraConsentChoice) = com.blent.MediaLocalConsent(
        choice.foreground, choice.permission, choice.enabled, choice.backgroundAllowed)
    private fun action(value: ConnectionMediaConsentAction<CameraPermissionRequest>?): CameraConsentAction? = when (value) {
        null -> null
        is ConnectionMediaConsentAction.Permission -> CameraConsentAction.Permission(value.request)
        ConnectionMediaConsentAction.Prepare -> CameraConsentAction.Prepare(offer)
        is ConnectionMediaConsentAction.Reject -> CameraConsentAction.Reject(offer.lease, value.outcome)
        is ConnectionMediaConsentAction.Cancel -> CameraConsentAction.Cancel(offer.lease, value.reason)
    }
    private fun validate(value: ConnectionSessionEvent.Offered): ConnectionManagement.Camera {
        ConnectionLocalLease.validate(value.lease, ConnectionManagement.Resource.CAMERA)
        val camera = value.profile as? ConnectionManagement.Camera
            ?: throw IllegalArgumentException("Camera consent requires camera profile")
        ConnectionManagement.encode(ConnectionManagement.Offer(value.lease.key, value.credential, camera))
        return camera
    }
}
