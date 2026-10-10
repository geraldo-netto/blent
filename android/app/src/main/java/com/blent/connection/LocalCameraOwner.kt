package com.blent.connection

/** One camera offer, consent policy, local endpoint group and independently
 * retained external capture-consumer claim. This class performs no blocking I/O
 * and cannot revoke sibling resources. Adopt it before calling open/advance. */
internal class LocalCameraOwner(
    override val offer: ConnectionSessionEvent.Offered,
    private val endpoints: ConnectionEndpoints,
    private val block: Int,
    private val limits: ConnectionRunLimits,
    private val authority: ConnectionResourceAuthority,
) : LocalResourceOwnership, LocalCameraControl, CameraConsentAuthority {
    private var previous = 0L
    private var factory: ((Long) -> LocalEndpoint)? = null
    private var group: LocalResourceGroup? = null
    private var choice = CameraConsentChoice(false, false)
    private var closedAt: Long? = null
    private var localGone = false
    private var consumerPossible = false
    private var consumerGone = false
    private var released = false
    private var uncertain = false
    private val consent = ConnectionCameraConsent(offer, this)
    private var publication = LocalCameraOffer(offer.lease, offer.profile as ConnectionManagement.Camera, false)
    private val guarded = object : ConnectionEndpoints by endpoints {
        override fun cancel(lease: ConnectionResourceLease, reason: ConnectionManagement.Reason) {
            check(lease == offer.lease)
            close(previous)
            endpoints.cancel(lease, reason)
        }
        override fun nativeRetired(lease: ConnectionResourceLease) {
            check(lease == offer.lease && !localGone)
            localGone = true
        }
    }
    init { require(block in 1..ConnectionWire.MAX_PAYLOAD) }

    private fun time(now: Long) {
        require(now >= previous && now <= Long.MAX_VALUE - maxOf(limits.ioTimeoutMs, limits.retirementTimeoutMs))
        previous = now
    }
    override fun owns(lease: ConnectionResourceLease): Boolean = factory != null && closedAt == null && lease == offer.lease && authority.owns(lease, previous)
    override fun admitted(lease: ConnectionResourceLease): Boolean = owns(lease) && endpoints.current(lease)
    override fun open(now: Long, factory: (Long) -> LocalEndpoint) {
        time(now); check(this.factory == null && closedAt == null)
        this.factory = factory
    }
    override fun advance(now: Long) {
        time(now)
        if (closedAt == null && factory != null) apply(consent.update(choice), now)
        group?.advance(now)
        release()
    }
    override fun offered(): LocalCameraOffer? {
        if (!owns(offer.lease) || endpoints.current(offer.lease)) return null
        val needed = consent.pending() != null
        if (publication.permissionRequired != needed) publication = publication.copy(permissionRequired = needed)
        return publication
    }
    override fun pending(): CameraPermissionRequest? = if (owns(offer.lease)) consent.pending() else null
    override fun camera(): LocalCameraRoute? {
        if (!owns(offer.lease)) return null
        val selected = group?.cameraRoute() ?: return null
        consumerPossible = true // Exposure precedes delivery, even if delivery fails.
        return selected
    }
    override fun choose(lease: ConnectionResourceLease, choice: CameraConsentChoice, now: Long): Boolean {
        time(now)
        if (!owns(lease)) return false
        this.choice = choice
        apply(consent.update(choice), now)
        release()
        return true
    }
    override fun permissionResult(prompt: CameraPermissionRequest, granted: Boolean, choice: CameraConsentChoice, now: Long): Boolean {
        time(now)
        if (!owns(prompt.lease) || consent.pending() !== prompt) return false
        this.choice = choice
        apply(consent.permissionResult(prompt, granted, choice), now)
        release()
        return true
    }
    private fun apply(action: CameraConsentAction?, now: Long) {
        when (action) {
            null -> Unit
            is CameraConsentAction.Permission -> endpoints.permissionRequired(action.request.lease, now)
            is CameraConsentAction.Prepare -> prepare(now)
            is CameraConsentAction.Reject -> { close(now); endpoints.reject(action.lease, action.outcome) }
            is CameraConsentAction.Cancel -> { close(now); endpoints.cancel(action.lease, action.reason) }
        }
    }
    private fun prepare(now: Long) {
        check(group == null && closedAt == null)
        group = LocalResourceGroup(offer, guarded, block, limits, CameraLocalPermit(consent) { choice })
        checkNotNull(group).open(now, checkNotNull(factory))
    }
    override fun admit(now: Long) {
        time(now)
        if (closedAt != null) return
        val native = checkNotNull(group) { "Camera has not been prepared" }
        native.admit(now)
    }
    private fun close(now: Long) {
        if (closedAt != null) return
        closedAt = now
        consent.stop()
        group?.stop(now)
        if (group == null) localGone = true
    }
    override fun stop(now: Long) {
        time(now)
        if (closedAt == null) { close(now); endpoints.cancel(offer.lease, ConnectionManagement.Reason.REQUESTED) }
        release()
    }
    override fun consumerRetired(lease: ConnectionResourceLease, now: Long): Boolean {
        time(now)
        if (lease != offer.lease || closedAt == null || !consumerPossible || consumerGone) return false
        consumerGone = true
        release()
        return true
    }
    private fun release() {
        if (released || uncertain || closedAt == null || !localGone) return
        if (consumerPossible && !consumerGone) return
        uncertain = true
        try { endpoints.nativeRetired(offer.lease); released = true; uncertain = false }
        catch (_: Exception) { /* Retain uncertain publication; never replay cleanup or free capacity. */ }
    }
    override fun retired(): Boolean = released && !uncertain && group?.retired() != false
    override fun quarantined(): Boolean {
        if (uncertain || group?.quarantined() == true) return true
        val stopped = closedAt ?: return false
        return consumerPossible && !consumerGone && previous - stopped >= limits.retirementTimeoutMs
    }
}
