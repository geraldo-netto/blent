package com.blent.connection

import com.blent.MediaLocalConsent

/** Audio route metadata is private loopback data, never a management address.
 * Interpretation grants no permission or original worker/lease authority. */
internal data class LocalAudioRoute(
    override val lease: ConnectionResourceLease,
    override val credential: ConnectionManagement.Credential,
    override val port: Int,
    override val profile: ConnectionManagement.Audio,
) : LocalMediaRoute<ConnectionManagement.Audio> {
    init { validateLocalAudio(lease, profile); require(port in 1..65535) }
}
internal data class LocalAudioOffer(override val lease: ConnectionResourceLease, override val profile: ConnectionManagement.Audio) : LocalMediaOffer<ConnectionManagement.Audio> {
    init { validateLocalAudio(lease, profile) }
}
internal object ConnectionAudioLease {
    fun validate(lease: ConnectionResourceLease) {
        require(lease.key.resource == ConnectionManagement.Resource.MICROPHONE || lease.key.resource == ConnectionManagement.Resource.SPEAKERS)
        ConnectionLocalLease.validate(lease, lease.key.resource)
    }
}
private fun validateLocalAudio(lease: ConnectionResourceLease, profile: ConnectionManagement.Audio) {
    ConnectionAudioLease.validate(lease)
    ConnectionManagement.encode(ConnectionManagement.Offer(lease.key, ConnectionManagement.Credential(ByteArray(32)), profile))
}

/** Exact original lease controls. Closing/rejection cannot substitute for actual
 * endpoint and external audio worker completion. Methods are prompt/nonblocking. */
internal interface LocalAudioProjection {
    fun audioOffered(resource: ConnectionManagement.Resource): LocalAudioOffer?
    fun audio(resource: ConnectionManagement.Resource): LocalAudioRoute?
}
internal interface LocalAudioControl : LocalAudioProjection {
    fun chooseAudio(lease: ConnectionResourceLease, choice: MediaLocalConsent, now: Long): Boolean
    fun audioConsumerRetired(lease: ConnectionResourceLease, now: Long): Boolean
}
