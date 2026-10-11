package com.blent.connection

/** Typed permission-free playback boundary. Native ownership and logical consent
 * are shared with microphone without allowing playback to request SDK permission. */
internal class LocalSpeakerOwner private constructor(private val owner: LocalAudioOwner) :
    LocalResourceOwnership by owner, LocalAudioControl by owner {
    constructor(offer: ConnectionSessionEvent.Offered, endpoints: ConnectionEndpoints, block: Int,
                limits: ConnectionRunLimits, authority: ConnectionResourceAuthority) :
        this(LocalAudioOwner(speaker(offer), endpoints, block, limits, authority))
}
private fun speaker(offer: ConnectionSessionEvent.Offered): ConnectionSessionEvent.Offered {
    require(offer.lease.key.resource == ConnectionManagement.Resource.SPEAKERS)
    return offer
}
