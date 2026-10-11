package com.blent.connection

/** Camera consent adapter around the shared exact media/native owner. */
internal class LocalCameraOwner(
    override val offer: ConnectionSessionEvent.Offered,
    private val endpoints: ConnectionEndpoints,
    block: Int,
    private val limits: ConnectionRunLimits,
    authority: ConnectionResourceAuthority,
) : LocalResourceOwnership, LocalCameraControl, CameraConsentAuthority {
    private var previous = 0L
    private var choice = CameraConsentChoice(false, false)
    private val consent = ConnectionCameraConsent(offer, this)
    private var publication = LocalCameraOffer(offer.lease, offer.profile as ConnectionManagement.Camera, false)
    private val native = LocalMediaResource(offer, endpoints, block, limits, authority,
        CameraLocalPermit(consent) { choice }, { consent.stop() })
    private fun time(now: Long) {
        require(now >= previous && now <= Long.MAX_VALUE - maxOf(limits.ioTimeoutMs, limits.retirementTimeoutMs))
        previous = now
    }
    override fun owns(lease: ConnectionResourceLease): Boolean = native.owns(lease, previous)
    override fun admitted(lease: ConnectionResourceLease): Boolean = native.admitted(lease, previous)
    override fun open(now: Long, factory: (Long) -> LocalEndpoint) { time(now); native.open(now, factory) }
    override fun advance(now: Long) {
        time(now)
        if (native.active()) apply(consent.update(choice), now)
        native.advance(now)
    }
    override fun offered(): LocalCameraOffer? {
        if (!owns(offer.lease) || endpoints.current(offer.lease)) return null
        val needed = consent.pending() != null
        if (publication.permissionRequired != needed) publication = publication.copy(permissionRequired = needed)
        return publication
    }
    override fun pending(): CameraPermissionRequest? = if (owns(offer.lease)) consent.pending() else null
    override fun camera(): LocalCameraRoute? = native.project(previous) { it.cameraRoute() }
    override fun choose(lease: ConnectionResourceLease, choice: CameraConsentChoice, now: Long): Boolean {
        time(now)
        if (!owns(lease)) return false
        this.choice = choice; apply(consent.update(choice), now); native.release(); return true
    }
    override fun permissionResult(prompt: CameraPermissionRequest, granted: Boolean, choice: CameraConsentChoice, now: Long): Boolean {
        time(now)
        if (!owns(prompt.lease) || consent.pending() !== prompt) return false
        this.choice = choice; apply(consent.permissionResult(prompt, granted, choice), now); native.release(); return true
    }
    private fun apply(action: CameraConsentAction?, now: Long) {
        when (action) {
            null -> Unit
            is CameraConsentAction.Permission -> endpoints.permissionRequired(action.request.lease, now)
            is CameraConsentAction.Prepare -> native.prepare(now)
            is CameraConsentAction.Reject -> { native.close(now); endpoints.reject(action.lease, action.outcome) }
            is CameraConsentAction.Cancel -> { native.close(now); endpoints.cancel(action.lease, action.reason) }
        }
    }
    override fun admit(now: Long) { time(now); native.admit(now) }
    override fun stop(now: Long) { time(now); native.stop(now) }
    override fun consumerRetired(lease: ConnectionResourceLease, now: Long): Boolean { time(now); return native.consumerRetired(lease, now) }
    override fun retired(): Boolean = native.retired()
    override fun quarantined(): Boolean = native.quarantined()
}
