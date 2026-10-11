package com.blent.connection

import com.blent.MediaLocalConsent

/** Audio route metadata is private loopback data, never a management address.
 * Interpretation grants no permission or original worker/lease authority. */
internal data class LocalAudioRoute(
    val lease: ConnectionResourceLease,
    val credential: ConnectionManagement.Credential,
    val port: Int,
    val profile: ConnectionManagement.Audio,
) {
    init { validateLocalAudio(lease, profile); require(port in 1..65535) }
}
internal data class LocalAudioOffer(val lease: ConnectionResourceLease, val profile: ConnectionManagement.Audio) {
    init { validateLocalAudio(lease, profile) }
}
private fun validateLocalAudio(lease: ConnectionResourceLease, profile: ConnectionManagement.Audio) {
    require(lease.key.resource == ConnectionManagement.Resource.MICROPHONE || lease.key.resource == ConnectionManagement.Resource.SPEAKERS)
    ConnectionLocalLease.validate(lease, lease.key.resource)
    ConnectionManagement.encode(ConnectionManagement.Offer(lease.key, ConnectionManagement.Credential(ByteArray(32)), profile))
}

/** Exact original lease controls. Closing/rejection cannot substitute for actual
 * endpoint and external audio worker completion. Methods are prompt/nonblocking. */
internal interface LocalAudioControl {
    fun audioOffered(resource: ConnectionManagement.Resource): LocalAudioOffer?
    fun audio(resource: ConnectionManagement.Resource): LocalAudioRoute?
    fun chooseAudio(lease: ConnectionResourceLease, choice: MediaLocalConsent, now: Long): Boolean
    fun audioConsumerRetired(lease: ConnectionResourceLease, now: Long): Boolean
}
