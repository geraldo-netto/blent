package com.blent

/** Fresh local intent and OS permission. A pairing grant or permission callback
 * cannot substitute for a fresh permission sample or background consent. */
internal data class CameraLocalConsent(
    val foreground: Boolean,
    val permission: Boolean,
    val enabled: Boolean,
    val backgroundAllowed: Boolean,
)

/** Authority for one exact publication. Boundary adapters retain original worker
 * and prompt identity; portable callers never reconstruct authority from values. */
internal interface CameraAccessAuthority {
    fun current(): Boolean
    fun update(context: CameraLocalConsent): Boolean
    fun permissionResult(granted: Boolean, context: CameraLocalConsent): Boolean
}

/** owner is an opaque original lifetime, stable across its offer/permission/route
 * publications, never a numeric lease or DTO equality. Contains no credential. */
internal class CameraAccessOffer(
    val owner: Any,
    val lens: CameraLens,
    val backgroundRequested: Boolean,
    val permissionRequired: Boolean,
    val admitted: Boolean,
    val authority: CameraAccessAuthority,
) {
    init { require(!permissionRequired || !admitted) }
}

internal data class CameraConsentView(
    val offer: CameraAccessOffer? = null,
    val enabled: Boolean = false,
    val foreground: Boolean = false,
    val backgroundAllowed: Boolean = false,
)

/** Focused local control contract; UI does not depend on policy implementation. */
internal interface CameraAccessControls {
    val state: kotlinx.coroutines.flow.StateFlow<CameraConsentView>
    fun start(offer: CameraAccessOffer): Boolean
    fun stop(offer: CameraAccessOffer): Boolean
    fun background(offer: CameraAccessOffer, allowed: Boolean): Boolean
}
