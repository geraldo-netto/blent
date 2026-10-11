package com.blent.connection

import com.blent.MediaLocalConsent

/** One locally selected speaker lease. No microphone SDK permission prompt is
 * valid for playback. Microphone composition retains a separate consent adapter.
 * Native cleanup and consumer exposure use the shared exact media owner. */
internal class LocalSpeakerOwner(
    override val offer: ConnectionSessionEvent.Offered,
    private val endpoints: ConnectionEndpoints,
    block: Int,
    private val limits: ConnectionRunLimits,
    authority: ConnectionResourceAuthority,
) : LocalResourceOwnership, LocalAudioControl, LocalResourcePermit {
    private var previous = 0L
    private val publication = validate(offer)
    private var choice = MediaLocalConsent(false, true, false, false)
    private var opening = false
    private val native = LocalMediaResource(offer, endpoints, block, limits, authority, this, { opening = false })
    private fun validate(value: ConnectionSessionEvent.Offered): LocalAudioOffer {
        require(value.lease.key.resource == ConnectionManagement.Resource.SPEAKERS)
        val profile = value.profile as? ConnectionManagement.Audio ?: throw IllegalArgumentException("Speaker profile required")
        ConnectionManagement.encode(ConnectionManagement.Offer(value.lease.key, value.credential, profile))
        return LocalAudioOffer(value.lease, profile)
    }
    private fun time(now: Long) {
        require(now >= previous && now <= Long.MAX_VALUE - maxOf(limits.ioTimeoutMs, limits.retirementTimeoutMs))
        previous = now
    }
    private fun allowed(): Boolean = choice.enabled && choice.permission &&
        (choice.foreground || (publication.profile.background && choice.backgroundAllowed))
    override fun opening(lease: ConnectionResourceLease): Boolean = opening && allowed() && native.owns(lease, previous)
    override fun admission(lease: ConnectionResourceLease): Boolean = opening(lease) && endpoints.current(lease)
    override fun open(now: Long, factory: (Long) -> LocalEndpoint) { time(now); native.open(now, factory) }
    override fun advance(now: Long) {
        time(now)
        if (native.active()) update(now)
        native.advance(now)
    }
    private fun update(now: Long) {
        if (!native.owns(offer.lease, now)) { close(now, ConnectionManagement.Reason.CONNECTION_LOST); return }
        if (opening) {
            if (!allowed()) close(now, if (!choice.permission) ConnectionManagement.Reason.PERMISSION_REVOKED else ConnectionManagement.Reason.REQUESTED)
        } else if (allowed()) { opening = true; native.prepare(now) }
    }
    private fun close(now: Long, reason: ConnectionManagement.Reason) {
        native.close(now); endpoints.cancel(offer.lease, reason)
    }
    override fun audioOffered(resource: ConnectionManagement.Resource): LocalAudioOffer? =
        if (resource == offer.lease.key.resource && native.owns(offer.lease, previous) && !endpoints.current(offer.lease)) publication else null
    override fun audio(resource: ConnectionManagement.Resource): LocalAudioRoute? =
        if (resource == offer.lease.key.resource) native.project(previous) { it.audioRoute() } else null
    override fun chooseAudio(lease: ConnectionResourceLease, choice: MediaLocalConsent, now: Long): Boolean {
        time(now)
        if (!native.owns(lease, now)) return false
        this.choice = choice
        if (!choice.enabled) {
            val prepared = opening
            native.close(now)
            if (prepared) endpoints.cancel(lease, ConnectionManagement.Reason.REQUESTED)
            else endpoints.reject(lease, ConnectionManagement.Outcome.UNAVAILABLE)
        } else update(now)
        native.release(); return true
    }
    override fun admit(now: Long) { time(now); native.admit(now) }
    override fun stop(now: Long) { time(now); native.stop(now) }
    override fun audioConsumerRetired(lease: ConnectionResourceLease, now: Long): Boolean {
        time(now); return native.consumerRetired(lease, now)
    }
    override fun retired(): Boolean = native.retired()
    override fun quarantined(): Boolean = native.quarantined()
}
