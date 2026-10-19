package com.blent.connection

/** One resource owns all its channels. Constructor performs no native work;
 * its controller must retain this object BEFORE calling open. */
internal class LocalResourceGroup(
    override val offer: ConnectionSessionEvent.Offered,
    private val endpoints: ConnectionEndpoints,
    private val block: Int,
    private val limits: ConnectionRunLimits,
    private val permit: LocalResourcePermit = DisplayLocalPermit,
) : LocalResourceOwnership, LocalDisplayProjection, ConnectionDeadlineSource {
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
    private var camera: LocalCameraRoute? = null
    private var audio: LocalAudioRoute? = null
    private var previous = 0L
    private var started = false
    init {
        require(block in 1..ConnectionWire.MAX_PAYLOAD)
        val resource = offer.lease.key.resource
        ConnectionLocalLease.validate(offer.lease, resource)
        ConnectionManagement.encode(ConnectionManagement.Offer(offer.lease.key, offer.credential, offer.profile))
    }

    private fun time(now: Long) {
        require(now >= previous && now <= Long.MAX_VALUE - maxOf(limits.ioTimeoutMs, limits.retirementTimeoutMs))
        previous = now
    }
    override fun open(now: Long, factory: (Long) -> LocalEndpoint) {
        time(now)
        check(!started && channels.isEmpty() && !uncertain && !closing)
        started = true
        try {
            if (!permit.opening(offer.lease)) { revoke(now); return }
            for (id in ConnectionChannels.members(offer.lease.key.resource)) {
                uncertain = true
                channels.add(Channel(id, factory(now)))
                uncertain = false
            }
        } catch (failure: Exception) { fail(failure, now) }
    }
    override fun advance(now: Long) {
        time(now)
        try {
            if (!closing && !permit.opening(offer.lease)) revoke(now)
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
        if (!started || prepared || channels.any { it.endpoint.port() == null }) return
        val ports = channels.map { checkNotNull(it.endpoint.port()) }
        require(ports.all { it in 1..65535 } && ports.distinct().size == ports.size)
        endpoints.prepared(offer.lease, now)
        prepared = true
    }
    override fun admit(now: Long) {
        if (closing) return
        time(now)
        try {
            check(prepared && !admitted && endpoints.current(offer.lease))
            if (!permit.admission(offer.lease)) { revoke(now); return }
            for (channel in channels) channel.endpoint.admit(now)
            publish()
            admitted = true
        } catch (failure: Exception) { fail(failure, now) }
    }
    private fun publish() {
        when (offer.lease.key.resource) {
            ConnectionManagement.Resource.DISPLAY -> invitation = LocalDisplayRoute(offer.lease, offer.credential,
                port(ConnectionWire.CONTROL), port(ConnectionWire.DISPLAY))
            ConnectionManagement.Resource.CAMERA -> camera = LocalCameraRoute(offer.lease, offer.credential,
                port(ConnectionWire.CAMERA), offer.profile as ConnectionManagement.Camera)
            ConnectionManagement.Resource.MICROPHONE, ConnectionManagement.Resource.SPEAKERS -> audio = LocalAudioRoute(
                offer.lease, offer.credential, port(ConnectionChannels.members(offer.lease.key.resource).single()),
                offer.profile as ConnectionManagement.Audio)
        }
    }
    private fun port(id: Int): Int = checkNotNull(channels.single { it.id == id }.endpoint.port())
    private fun visible(): Boolean = try {
        !closing && endpoints.current(offer.lease) && permit.admission(offer.lease)
    } catch (failure: Exception) {
        fail(failure, previous)
        false
    }
    override fun route(): LocalDisplayRoute? = if (visible()) invitation else null
    fun cameraRoute(): LocalCameraRoute? = if (visible()) camera else null
    fun audioRoute(): LocalAudioRoute? = if (visible()) audio else null
    private fun revoke(now: Long) {
        stop(now)
        endpoints.cancel(offer.lease, ConnectionManagement.Reason.PERMISSION_REVOKED)
    }
    private fun fail(failure: Exception, now: Long) {
        if (error == null) error = failure
        stop(now)
        endpoints.cancel(offer.lease, ConnectionManagement.Reason.WORKER_FAILED)
    }
    override fun stop(now: Long) {
        time(now)
        if (closing) return
        closing = true; invitation = null; camera = null; audio = null
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
    override fun retired(): Boolean = closing && !uncertain && channels.all { it.endpoint.retired() && it.pump?.retired() != false }
    override fun quarantined(): Boolean = uncertain || channels.any { it.endpoint.quarantined() || it.pump?.quarantined() == true }
    fun failure(): Exception? = error
    override fun nextDeadline(): ConnectionDeadline {
        if (retired()) return ConnectionDeadline.IDLE
        var deadline = if (uncertain) ConnectionDeadline.UNKNOWN else ConnectionDeadline.IDLE
        for (channel in channels) {
            if (!channel.endpoint.retired()) {
                deadline = deadline.combine(ConnectionDeadline.observe(channel.endpoint as? ConnectionDeadlineSource))
            }
            val pump = channel.pump
            if (pump != null) deadline = deadline.combine(pump.nextDeadline())
        }
        return deadline
    }
}
