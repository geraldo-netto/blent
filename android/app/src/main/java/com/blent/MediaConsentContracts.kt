package com.blent

/** Original publication metadata, independent of camera/audio implementation,
 * connection IDs, credentials, workers and OS device handles. */
internal interface MediaAccessOffer {
    val owner: Any
    val backgroundRequested: Boolean
    val permissionRequired: Boolean
    val admitted: Boolean
    val authority: MediaAccessAuthority
}

/** Fresh local intent and OS permission. A pairing grant or permission callback
 * cannot substitute for a fresh permission sample or background consent. */
internal data class MediaLocalConsent(
    val foreground: Boolean,
    val permission: Boolean,
    val enabled: Boolean,
    val backgroundAllowed: Boolean,
)

/** Authority for one exact publication. Boundary adapters retain original worker
 * and prompt identity; portable callers never reconstruct authority from values. */
internal interface MediaAccessAuthority {
    fun current(): Boolean
    fun update(context: MediaLocalConsent): Boolean
    fun permissionResult(granted: Boolean, context: MediaLocalConsent): Boolean
}

internal data class MediaConsentView<O : MediaAccessOffer>(
    val offer: O? = null,
    val enabled: Boolean = false,
    val foreground: Boolean = false,
    val backgroundAllowed: Boolean = false,
)

/** Focused local control contract; UI does not depend on policy implementation. */
internal interface MediaAccessControls<O : MediaAccessOffer> {
    val state: kotlinx.coroutines.flow.StateFlow<MediaConsentView<O>>
    fun start(offer: O): Boolean
    fun stop(offer: O): Boolean
    fun background(offer: O, allowed: Boolean): Boolean
}
