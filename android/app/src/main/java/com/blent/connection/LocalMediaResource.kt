package com.blent.connection

/** Shared native ownership for one consent-controlled media lease. Publication
 * exposure, logical closure and actual native/consumer completion stay separate.
 * Policy and OS adapters remain outside this owner; a stuck lease cannot close
 * siblings. Adopt this owner before opening its lazy endpoint factory. */
internal class LocalMediaResource(
    val offer: ConnectionSessionEvent.Offered,
    private val endpoints: ConnectionEndpoints,
    private val block: Int,
    private val limits: ConnectionRunLimits,
    private val authority: ConnectionResourceAuthority,
    private val permit: LocalResourcePermit,
    private val onClosed: () -> Unit,
) {
    private var previous = 0L
    private var factory: ((Long) -> LocalEndpoint)? = null
    private var group: LocalResourceGroup? = null
    private var closedAt: Long? = null
    private var localGone = false
    private var consumerPossible = false
    private var consumerGone = false
    private var released = false
    private var uncertain = false
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
    fun active(): Boolean = factory != null && closedAt == null
    fun owns(lease: ConnectionResourceLease, now: Long): Boolean =
        active() && lease == offer.lease && authority.owns(lease, now)
    fun admitted(lease: ConnectionResourceLease, now: Long): Boolean = owns(lease, now) && endpoints.current(lease)
    fun open(now: Long, factory: (Long) -> LocalEndpoint) {
        time(now); check(this.factory == null && closedAt == null)
        this.factory = factory
    }
    fun prepare(now: Long) {
        time(now); check(group == null && closedAt == null)
        group = LocalResourceGroup(offer, guarded, block, limits, permit)
        checkNotNull(group).open(now, checkNotNull(factory))
    }
    fun advance(now: Long) { time(now); group?.advance(now); release() }
    fun admit(now: Long) {
        time(now)
        if (closedAt != null) return
        checkNotNull(group) { "Media has not been prepared" }.admit(now)
    }
    fun <R> project(now: Long, read: (LocalResourceGroup) -> R?): R? {
        if (!owns(offer.lease, now)) return null
        val selected = group?.let(read) ?: return null
        consumerPossible = true // Exposure retained even when delivery fails.
        return selected
    }
    fun close(now: Long) {
        time(now)
        if (closedAt != null) return
        closedAt = now
        onClosed()
        group?.stop(now)
        if (group == null) localGone = true
    }
    fun stop(now: Long) {
        time(now)
        if (closedAt == null) { close(now); endpoints.cancel(offer.lease, ConnectionManagement.Reason.REQUESTED) }
        release()
    }
    fun consumerRetired(lease: ConnectionResourceLease, now: Long): Boolean {
        time(now)
        if (lease != offer.lease || closedAt == null || !consumerPossible || consumerGone) return false
        consumerGone = true; release(); return true
    }
    fun release() {
        if (released || uncertain || closedAt == null || !localGone) return
        if (consumerPossible && !consumerGone) return
        uncertain = true
        try { endpoints.nativeRetired(offer.lease); released = true; uncertain = false }
        catch (_: Exception) { /* Uncertain publication never frees capacity. */ }
    }
    fun retired(): Boolean = released && !uncertain && group?.retired() != false
    fun quarantined(): Boolean {
        if (uncertain || group?.quarantined() == true) return true
        val stopped = closedAt ?: return false
        return consumerPossible && !consumerGone && previous - stopped >= limits.retirementTimeoutMs
    }
}
