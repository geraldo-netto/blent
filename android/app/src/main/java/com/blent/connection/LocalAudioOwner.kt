package com.blent.connection

import com.blent.MediaLocalConsent

/** One independently retained audio lease. Shared consent policy grants logical
 * transitions; exact native endpoint/consumer proof owns retirement. The caller
 * explicitly composes a microphone authority before advertising that capability. */
internal class LocalAudioOwner(
    override val offer: ConnectionSessionEvent.Offered,
    private val endpoints: ConnectionEndpoints,
    block: Int,
    private val limits: ConnectionRunLimits,
    authority: ConnectionResourceAuthority,
) : LocalResourceOwnership, LocalAudioControl, LocalAudioPermissionControl,
    LocalResourcePermit, ConnectionMediaConsentAuthority, ConnectionDeadlineSource {
    private var previous = 0L
    private var selected = false
    private var publication = validate(offer)
    private var choice = MediaLocalConsent(false, offer.lease.key.resource == ConnectionManagement.Resource.SPEAKERS, false, false)
    private val consent = ConnectionMediaConsent(offer.lease, publication.profile.background,
        offer.lease.key.resource == ConnectionManagement.Resource.MICROPHONE, this, ::AudioPermissionRequest)
    private val native = LocalMediaResource(offer, endpoints, block, limits, authority, this, { consent.stop() })
    private fun validate(value: ConnectionSessionEvent.Offered): LocalAudioOffer {
        val profile = value.profile as? ConnectionManagement.Audio ?: throw IllegalArgumentException("Audio profile required")
        ConnectionManagement.encode(ConnectionManagement.Offer(value.lease.key, value.credential, profile))
        return LocalAudioOffer(value.lease, profile)
    }
    private fun time(now: Long) {
        require(now >= previous && now <= Long.MAX_VALUE - maxOf(limits.ioTimeoutMs, limits.retirementTimeoutMs))
        previous = now
    }
    override fun owns(lease: ConnectionResourceLease): Boolean = native.owns(lease, previous)
    override fun admitted(lease: ConnectionResourceLease): Boolean = native.admitted(lease, previous)
    override fun opening(lease: ConnectionResourceLease): Boolean = consent.preparationAllowed(lease, choice)
    override fun admission(lease: ConnectionResourceLease): Boolean = consent.captureAllowed(lease, choice)
    override fun open(now: Long, factory: (Long) -> LocalEndpoint) { time(now); native.open(now, factory) }
    override fun advance(now: Long) {
        time(now)
        if (native.active()) update(now)
        native.advance(now)
    }
    private fun update(now: Long) {
        if (!owns(offer.lease)) apply(consent.connectionLost(), now)
        else if (selected) apply(consent.update(choice), now)
    }
    override fun audioOffered(resource: ConnectionManagement.Resource): LocalAudioOffer? {
        if (resource != offer.lease.key.resource || !owns(offer.lease) || endpoints.current(offer.lease)) return null
        val needed = consent.pending() != null
        if (publication.permissionRequired != needed) publication = publication.copy(permissionRequired = needed)
        return publication
    }
    override fun audio(resource: ConnectionManagement.Resource): LocalAudioRoute? =
        if (resource == offer.lease.key.resource) native.project(previous) { it.audioRoute() } else null
    override fun chooseAudio(lease: ConnectionResourceLease, choice: MediaLocalConsent, now: Long): Boolean {
        time(now)
        if (!owns(lease)) return false
        selected = true
        this.choice = choice; apply(consent.update(choice), now); native.release(); return true
    }
    override fun pendingAudio(): AudioPermissionRequest? = if (owns(offer.lease)) consent.pending() else null
    override fun audioPermissionResult(prompt: AudioPermissionRequest, granted: Boolean, choice: MediaLocalConsent, now: Long): Boolean {
        time(now)
        if (!owns(prompt.lease) || consent.pending() !== prompt) return false
        this.choice = choice; apply(consent.permissionResult(prompt, granted, choice), now); native.release(); return true
    }
    private fun apply(action: ConnectionMediaConsentAction<AudioPermissionRequest>?, now: Long) {
        when (action) {
            null -> Unit
            is ConnectionMediaConsentAction.Permission -> endpoints.permissionRequired(action.request.lease, now)
            ConnectionMediaConsentAction.Prepare -> native.prepare(now)
            is ConnectionMediaConsentAction.Reject -> { native.close(now); endpoints.reject(offer.lease, action.outcome) }
            is ConnectionMediaConsentAction.Cancel -> { native.close(now); endpoints.cancel(offer.lease, action.reason) }
        }
    }
    override fun admit(now: Long) { time(now); native.admit(now) }
    override fun stop(now: Long) { time(now); native.stop(now) }
    override fun audioConsumerRetired(lease: ConnectionResourceLease, now: Long): Boolean {
        time(now); return native.consumerRetired(lease, now)
    }
    override fun retired(): Boolean = native.retired()
    override fun quarantined(): Boolean = native.quarantined()
    override fun nextDeadline(): ConnectionDeadline = native.nextDeadline()
}
