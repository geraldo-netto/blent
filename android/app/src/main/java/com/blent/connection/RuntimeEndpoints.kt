package com.blent.connection

/** Admission checks are separate from exact cleanup receipts: revocation must
 * not strand a native writer that still owns a borrowed ring segment. */
internal class RuntimeEndpoints(private val owner: ConnectionRuntimeOwner) : ConnectionEndpoints {
    private fun admitted(lease: ConnectionResourceLease): ConnectionEndpoints {
        check(owner.admission(lease)) { "Connection approval is not current" }
        return checkNotNull(owner.raw())
    }

    override fun nextEvent(): ConnectionSessionEvent? = owner.endpointEvent()
    override fun prepared(lease: ConnectionResourceLease, now: Long) {
        val time = owner.endpointTime(now)
        admitted(lease).prepared(lease, time)
    }
    override fun permissionRequired(lease: ConnectionResourceLease, now: Long) {
        val time = owner.endpointTime(now)
        admitted(lease).permissionRequired(lease, time)
    }
    override fun reject(lease: ConnectionResourceLease, outcome: ConnectionManagement.Outcome) = admitted(lease).reject(lease, outcome)
    override fun cancel(lease: ConnectionResourceLease, reason: ConnectionManagement.Reason) = owner.retained(lease).cancel(lease, reason)
    override fun current(lease: ConnectionResourceLease): Boolean = owner.admission(lease) && owner.retained(lease).current(lease)
    override fun offer(lease: ConnectionResourceLease, channel: Int, bytes: ByteArray): Boolean =
        owner.admission(lease) && owner.retained(lease).offer(lease, channel, bytes)
    override fun read(lease: ConnectionResourceLease, channel: Int, maxBytes: Int): ConnectionChannelRead? =
        if (owner.admission(lease)) owner.retained(lease).read(lease, channel, maxBytes) else null
    override fun consumed(lease: ConnectionResourceLease, channel: Int, receipt: ConnectionChannelRead) = owner.retained(lease).consumed(lease, channel, receipt)
    override fun nativeRetired(lease: ConnectionResourceLease) = owner.retained(lease).nativeRetired(lease)
}
