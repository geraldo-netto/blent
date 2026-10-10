package com.blent.connection

/** Private local invitation, valid only while the exact lease remains current.
 * Addresses are fixed loopback; ports originate in local listeners, never wire
 * metadata. Reading this value conveys neither pairing nor media authority. */
internal data class LocalDisplayRoute(
    val lease: ConnectionResourceLease,
    val credential: ConnectionManagement.Credential,
    val controlPort: Int,
    val displayPort: Int,
) {
    init {
        require(lease.key.resource == ConnectionManagement.Resource.DISPLAY)
        require(lease.owner > 0 && lease.connection.epoch > 0)
        require(lease.key.request > 0 && lease.key.generation > 0)
        require(controlPort in 1..65535 && displayPort in 1..65535 && controlPort != displayPort)
    }
}

/** One resource owns all its channels. Constructor performs no native work;
 * its controller must retain this object BEFORE calling open. */
internal class LocalResourceGroup(
    val offer: ConnectionSessionEvent.Offered,
    private val endpoints: ConnectionEndpoints,
    private val block: Int,
    private val limits: ConnectionRunLimits,
) {
    private class Channel(val id: Int, val endpoint: LocalEndpoint) {
        var pump: ConnectionLocalPump? = null
    }
    private val channels = ArrayList<Channel>(2)
    private var uncertain = false
    private var prepared = false
    private var admitted = false
    private var closing = false
    private var released = false
    private var error: Exception? = null
    private var invitation: LocalDisplayRoute? = null

    fun open(now: Long, factory: (Long) -> LocalEndpoint) {
        check(channels.isEmpty() && !uncertain && !closing)
        try {
            for (id in ConnectionChannels.members(offer.lease.key.resource)) {
                uncertain = true
                channels.add(Channel(id, factory(now)))
                uncertain = false
            }
        } catch (failure: Exception) { fail(failure, now) }
    }
    fun advance(now: Long) {
        try {
            for (channel in channels) step(channel, now)
            if (!closing) prepare(now)
        } catch (failure: Exception) { fail(failure, now) }
        release()
    }
    private fun step(channel: Channel, now: Long) {
        channel.endpoint.advance(now)
        if (!closing) channel.endpoint.failure()?.let { throw it }
        if (admitted && !closing && channel.pump == null) {
            channel.endpoint.takeIo()?.let {
                channel.pump = ConnectionLocalPump(endpoints, offer.lease, channel.id, it, block, limits)
            }
        }
        channel.pump?.advance(now)
        if (!closing) channel.pump?.failure()?.let { throw it }
    }
    private fun prepare(now: Long) {
        if (prepared || channels.any { it.endpoint.port() == null }) return
        val ports = channels.map { checkNotNull(it.endpoint.port()) }
        require(ports.all { it in 1..65535 } && ports.distinct().size == ports.size)
        endpoints.prepared(offer.lease, now)
        prepared = true
    }
    fun admit(now: Long) {
        if (closing) return
        try {
            check(prepared && !admitted && endpoints.current(offer.lease))
            for (channel in channels) channel.endpoint.admit(now)
            invitation = LocalDisplayRoute(offer.lease, offer.credential,
                port(ConnectionWire.CONTROL), port(ConnectionWire.DISPLAY))
            admitted = true
        } catch (failure: Exception) { fail(failure, now) }
    }
    private fun port(id: Int): Int = checkNotNull(channels.single { it.id == id }.endpoint.port())
    fun route(): LocalDisplayRoute? = if (endpoints.current(offer.lease)) invitation else null
    private fun fail(failure: Exception, now: Long) {
        if (error == null) error = failure
        stop(now)
        endpoints.cancel(offer.lease, ConnectionManagement.Reason.WORKER_FAILED)
    }
    fun stop(now: Long) {
        if (closing) return
        closing = true; invitation = null
        for (channel in channels) stopChannel(channel, now)
    }
    private fun stopChannel(channel: Channel, now: Long) {
        try { channel.pump?.stop(now) } catch (failure: Exception) { uncertain = true; error = failure }
        try { channel.endpoint.stop(now) } catch (failure: Exception) { uncertain = true; error = failure }
    }
    private fun release() {
        if (released || !closing || !retired()) return
        endpoints.nativeRetired(offer.lease)
        released = true
    }
    fun retired(): Boolean = closing && !uncertain && channels.all { it.endpoint.retired() && it.pump?.retired() != false }
    fun quarantined(): Boolean = uncertain || channels.any { it.endpoint.quarantined() || it.pump?.quarantined() == true }
    fun failure(): Exception? = error
}
