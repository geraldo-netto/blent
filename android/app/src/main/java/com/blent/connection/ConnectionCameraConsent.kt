package com.blent.connection

/** Authenticated ownership is checked for pending offers as well as admission.
 * Proof includes the original runtime/worker lifetime even if counters repeat.
 * Native composition supplies these proofs; a parsed lease supplies neither. */
internal interface CameraConsentAuthority {
    fun owns(lease: ConnectionResourceLease): Boolean
    fun admitted(lease: ConnectionResourceLease): Boolean
}

/** Snapshot of local intent and current OS permission, supplied by the UI adapter.
 * A granted callback alone is insufficient proof that permission still exists. */
internal data class CameraConsentChoice(
    val foreground: Boolean,
    val permission: Boolean,
    val enabled: Boolean = true,
    val backgroundAllowed: Boolean = false,
) {
    fun canRun(profile: ConnectionManagement.Camera): Boolean =
        enabled && permission && (foreground || (profile.background && backgroundAllowed))
}

/** Opaque prompt identity. Even an equal lease in another owner cannot reuse it.
 * IPC adapters must resolve replies through their retained exact worker/prompt. */
internal class CameraPermissionRequest(val lease: ConnectionResourceLease)

internal sealed interface CameraConsentAction {
    data class Permission(val request: CameraPermissionRequest) : CameraConsentAction
    data class Prepare(val offer: ConnectionSessionEvent.Offered) : CameraConsentAction
    data class Reject(val lease: ConnectionResourceLease, val outcome: ConnectionManagement.Outcome) : CameraConsentAction
    data class Cancel(val lease: ConnectionResourceLease, val reason: ConnectionManagement.Reason) : CameraConsentAction
}

/** One serialized owner per offered camera lease. Emits preparation only after
 * local consent. Closing revokes permission immediately, but this object and any
 * native endpoint must remain owned until the external actual-retirement proof. */
internal class ConnectionCameraConsent(
    val offer: ConnectionSessionEvent.Offered,
    private val authority: CameraConsentAuthority,
) {
    private val profile = validate(offer)
    private var request: CameraPermissionRequest? = null
    private var opening = false
    private var closed = false

    fun pending(): CameraPermissionRequest? = request
    fun closing(): Boolean = closed

    fun update(choice: CameraConsentChoice): CameraConsentAction? {
        if (closed) return null
        if (!authority.owns(offer.lease)) return connectionLost()
        if (!choice.enabled) return stop()
        if (opening) return maintain(choice)
        return prepare(choice)
    }

    private fun maintain(choice: CameraConsentChoice): CameraConsentAction? {
        if (!choice.permission) return cancel(ConnectionManagement.Reason.PERMISSION_REVOKED)
        if (!choice.canRun(profile)) return stop()
        return null
    }

    private fun prepare(choice: CameraConsentChoice): CameraConsentAction? {
        if (choice.canRun(profile)) {
            opening = true
            request = null
            return CameraConsentAction.Prepare(offer)
        }
        if (!choice.foreground || request != null) return null
        return CameraPermissionRequest(offer.lease).let {
            request = it
            CameraConsentAction.Permission(it)
        }
    }

    fun permissionResult(prompt: CameraPermissionRequest, granted: Boolean, choice: CameraConsentChoice): CameraConsentAction? {
        if (closed || prompt !== request) return null
        request = null
        if (!authority.owns(offer.lease)) return connectionLost()
        if (!choice.foreground || !choice.enabled) return stop()
        if (!granted || !choice.permission) return reject(ConnectionManagement.Outcome.PERMISSION_DENIED)
        return update(choice)
    }

    /** Worker opening also requires actual protocol admission, independently of
     * permission/preparation. Late worker and stale lease callbacks fail closed. */
    fun preparationAllowed(lease: ConnectionResourceLease, choice: CameraConsentChoice): Boolean =
        !closed && opening && lease == offer.lease && choice.canRun(profile) && authority.owns(lease)

    fun captureAllowed(lease: ConnectionResourceLease, choice: CameraConsentChoice): Boolean =
        preparationAllowed(lease, choice) && authority.admitted(lease)

    fun stop(): CameraConsentAction? {
        if (closed) return null
        return if (opening) cancel(ConnectionManagement.Reason.REQUESTED)
            else reject(ConnectionManagement.Outcome.UNAVAILABLE)
    }

    fun connectionLost(): CameraConsentAction? {
        if (closed) return null
        return cancel(ConnectionManagement.Reason.CONNECTION_LOST)
    }

    private fun cancel(reason: ConnectionManagement.Reason): CameraConsentAction {
        close()
        return CameraConsentAction.Cancel(offer.lease, reason)
    }
    private fun reject(outcome: ConnectionManagement.Outcome): CameraConsentAction {
        close()
        return CameraConsentAction.Reject(offer.lease, outcome)
    }
    private fun close() { closed = true; request = null }

    private fun validate(value: ConnectionSessionEvent.Offered): ConnectionManagement.Camera {
        val lease = value.lease
        require(lease.key.resource == ConnectionManagement.Resource.CAMERA)
        require(lease.owner > 0 && lease.connection.epoch > 0)
        val camera = value.profile as? ConnectionManagement.Camera
            ?: throw IllegalArgumentException("Camera consent requires camera profile")
        // Reuse the wire schema's complete constructed-value validation. Bounded
        // allocation once per offer, with no credentials in diagnostics.
        ConnectionManagement.encode(ConnectionManagement.Offer(lease.key, value.credential, camera))
        return camera
    }
}
