package com.blent.connection

import com.blent.connection.ConnectionManagement.Capabilities
import com.blent.connection.ConnectionManagement.Commit
import com.blent.connection.ConnectionManagement.Key
import com.blent.connection.ConnectionManagement.Message
import com.blent.connection.ConnectionManagement.Offer
import com.blent.connection.ConnectionManagement.Outcome
import com.blent.connection.ConnectionManagement.Prepared
import com.blent.connection.ConnectionManagement.Reason
import com.blent.connection.ConnectionManagement.ResultMessage
import com.blent.connection.ConnectionManagement.Revoke
import java.util.ArrayDeque

/** T882 single serialized tablet resource owner. Ticket must come from the
 * current authenticated pairing and capacity from completed TLS BLCP exchange.
 * Constructor/wire values alone do not authenticate a connection. Native device
 * and local endpoint operations belong to the supervised adapter, not this class. */
internal class TabletSession(
    ticket: ConnectionTicket, private val limits: ConnectionSessionLimits,
    private val capabilities: Int,
) : ConnectionSession {
    private class Binding(val lease: ConnectionResourceLease) {
        var installed = false
        var closing = false
        var nativeRetired = false
        var consumersRetired = false
        var channelsRetired = false
        var ackQueued = false
        var acknowledged = false
        var deferred: Offer? = null
    }
    init { require(ticket.epoch > 0 && capabilities in 0..15) }
    private val resources = ConnectionResources(ConnectionResources.Role.TABLET, ticket)
    private val mux = ConnectionMux(1, 1, limits.capacity, limits.messages, limits.quantum)
    private val channels = ConnectionChannelBuffers(limits.capacity)
    private val decoder = ManagementDecoder()
    private val publications = ConnectionPublications()
    private val slots = arrayOfNulls<Binding>(4)
    private val events = ArrayDeque<ConnectionSessionEvent>()
    private val readable = arrayOfNulls<ConnectionResourceLease>(5)
    private var cursor = 0
    private var owner = 0L
    private var connected = true
    private var transportGone = false
    private var writer: ConnectionFrame? = null
    init { publications.queue(Capabilities(capabilities)) }

    private fun binding(lease: ConnectionResourceLease): Binding {
        val binding = checkNotNull(slots[lease.key.resource.ordinal]) { "Unknown resource lease" }
        require(binding.lease == lease) { "Stale resource lease" }
        return binding
    }
    private fun lease(key: Key): ConnectionResourceLease {
        val lease = checkNotNull(slots[key.resource.ordinal]).lease
        require(lease.key == key) { "Stale management key" }
        return lease
    }
    private fun room() {
        check(connected) { "Connection closed" }
        publications.room()
        check(events.size < 28) { "Session event consumer stalled" }
    }
    private fun notice(lease: ConnectionResourceLease, state: ConnectionResourceState) {
        // Normal admission stops at 28; four slots each need Closing and Retired.
        check(events.size < 36) { "Session event capacity exhausted" }
        events.addLast(ConnectionSessionEvent.State(lease, state))
    }
    override fun current(lease: ConnectionResourceLease): Boolean = connected && resources.current(lease)

    override fun receive(frame: ConnectionFrame, now: Long) {
        check(connected)
        try {
            require(now >= 0 && frame.offset in 0..frame.bytes.size)
            require(frame.length == frame.header.length && frame.length <= frame.bytes.size - frame.offset)
            val deliver = mux.receive(frame.header)
            when (frame.header.channel) {
                ConnectionWire.MANAGEMENT -> management(frame, deliver, now)
                else -> media(frame, deliver)
            }
            settle(now)
        } catch (error: Exception) { disconnect(); throw error }
    }
    private fun management(frame: ConnectionFrame, deliver: Boolean, now: Long) {
        check(frame.header.kind == ConnectionWire.DATA || frame.header.kind == ConnectionWire.CREDIT)
        if (!deliver) return
        var offset = frame.offset
        val end = frame.offset + frame.length
        while (offset < end) {
            val decoded = decoder.feed(frame.bytes, offset, end - offset)
            check(decoded.consumed > 0)
            offset += decoded.consumed
            decoded.message?.let { message(it, now) }
        }
        mux.release(ConnectionWire.MANAGEMENT, frame.length)
    }
    private fun message(message: Message, now: Long) {
        when (message) {
            is Offer -> offered(message, now)
            is Commit -> install(lease(message.key), now)
            is Revoke -> {
                val lease = lease(message.key)
                resources.peerRevoke(lease)
                close(lease)
            }
            else -> error("Unexpected host management record")
        }
    }
    private fun offered(offer: Offer, now: Long) {
        room()
        val previous = slots[offer.key.resource.ordinal]
        if (previous != null) { defer(previous, offer); return }
        check(owner < Long.MAX_VALUE) { "Local resource owners exhausted" }
        val lease = resources.offer(offer.key, owner + 1, now, limits.resourceTimeoutMs)
        owner += 1
        slots[offer.key.resource.ordinal] = Binding(lease)
        if (capabilities and (1 shl offer.key.resource.ordinal) == 0) {
            reject(lease, Outcome.UNAVAILABLE)
            nativeRetired(lease) // No endpoint invitation or native operation was issued.
        } else {
            events.addLast(ConnectionSessionEvent.Offered(lease, offer.credential, offer.profile))
        }
    }
    private fun defer(previous: Binding, offer: Offer) {
        check(previous.ackQueued && !previous.acknowledged && previous.deferred == null)
        val old = previous.lease.key
        val generation = if (previous.installed && old.generation < Long.MAX_VALUE) old.generation + 1 else old.generation
        check(!previous.installed || old.generation < Long.MAX_VALUE)
        require(offer.key.generation == generation && offer.key.request > old.request)
        previous.deferred = offer
    }
    override fun prepared(lease: ConnectionResourceLease, now: Long) {
        binding(lease)
        room()
        check(resources.prepared(lease, now))
        publications.queue(Prepared(lease.key))
    }
    override fun permissionRequired(lease: ConnectionResourceLease, now: Long) {
        binding(lease)
        room()
        check(resources.permissionRequired(lease, now))
        publications.queue(ResultMessage(lease.key, Outcome.PERMISSION_REQUIRED))
    }
    override fun reject(lease: ConnectionResourceLease, outcome: Outcome) {
        binding(lease)
        room()
        resources.reject(lease, outcome)
        publications.queue(ResultMessage(lease.key, outcome))
        close(lease)
    }
    private fun install(lease: ConnectionResourceLease, now: Long) {
        room()
        val binding = binding(lease)
        val install = resources.commit(lease, now)
        binding.installed = true
        channels.install(lease.key.resource)
        for (channel in ConnectionChannels.members(lease.key.resource)) {
            if (install == ConnectionResources.Install.ACTIVATE) mux.activate(channel, lease.key.generation)
            else mux.close(channel, ConnectionWire.CLOSE)
        }
        if (install == ConnectionResources.Install.ACTIVATE) {
            check(resources.admitted(lease, now))
            publications.queue(ResultMessage(lease.key, Outcome.ADMITTED))
            notice(lease, ConnectionResourceState.ADMITTED)
        } else cancel(lease, Reason.TIMED_OUT)
        retireConsumers(binding)
    }
    private fun media(frame: ConnectionFrame, deliver: Boolean) {
        val channel = frame.header.channel
        val resource = ConnectionChannels.resource(channel)
        val binding = checkNotNull(slots[resource.ordinal])
        check(binding.installed)
        if (frame.header.kind == ConnectionWire.CLOSE || frame.header.kind == ConnectionWire.RESET) {
            cancel(binding.lease, Reason.WORKER_FAILED)
        } else if (deliver) {
            check(current(binding.lease))
            channels.push(channel, frame.bytes, frame.offset, frame.length)
            readable[channel - 1] = binding.lease
        }
    }
    override fun cancel(lease: ConnectionResourceLease, reason: Reason) {
        if (binding(lease).closing) return
        room()
        close(lease)
        publications.queue(Revoke(lease.key, reason))
    }
    private fun close(lease: ConnectionResourceLease) {
        val binding = binding(lease)
        if (binding.closing) return
        resources.revoke(lease)
        binding.closing = true
        if (binding.installed) {
            for (channel in ConnectionChannels.members(lease.key.resource)) {
                if (mux.acceptsData(channel, lease.key.generation)) mux.close(channel, ConnectionWire.CLOSE)
            }
        }
        notice(lease, ConnectionResourceState.CLOSING)
    }
    override fun nextFrame(now: Long): ConnectionFrame? {
        check(connected && now >= 0)
        if (writer != null) return null
        publications.publish(mux)
        val frame = mux.next() ?: return null
        return ConnectionFrame(frame.header, frame.bytes, frame.offset, frame.length).also { writer = it }
    }
    override fun written(frame: ConnectionFrame, now: Long) {
        require(writer === frame) { "Foreign or retired frame completion" }
        writer = null
        try {
            mux.finish(connected)
            if (!connected) return
            require(now >= 0)
            publications.written(frame.header)?.let { acknowledged(it) }
            settle(now)
        } catch (error: Exception) { disconnect(); throw error }
    }
    private fun acknowledged(lease: ConnectionResourceLease) {
        val binding = binding(lease)
        resources.acknowledge(lease)
        binding.acknowledged = true
    }
    override fun offer(lease: ConnectionResourceLease, channel: Int, bytes: ByteArray): Boolean {
        if (!current(lease) || !ConnectionChannels.belongs(lease.key.resource, channel)) return false
        return mux.offer(channel, bytes)
    }
    override fun read(lease: ConnectionResourceLease, channel: Int, maxBytes: Int): ConnectionChannelRead? {
        require(current(lease) && ConnectionChannels.belongs(lease.key.resource, channel))
        return channels.read(channel, maxBytes)
    }
    override fun consumed(lease: ConnectionResourceLease, channel: Int, receipt: ConnectionChannelRead) {
        binding(lease)
        require(ConnectionChannels.belongs(lease.key.resource, channel))
        channels.consumed(channel, receipt)
        if (current(lease)) {
            mux.release(channel, receipt.length)
            if (channels.readable(channel)) readable[channel - 1] = lease
        }
    }
    override fun nativeRetired(lease: ConnectionResourceLease) {
        val binding = binding(lease)
        resources.nativeRetired(lease)
        binding.nativeRetired = true
        retireConsumers(binding)
        settle(0)
    }
    private fun retireConsumers(binding: Binding) {
        if (!binding.installed || !binding.nativeRetired || binding.consumersRetired) return
        channels.retire(binding.lease.key.resource)
        if (connected) {
            for (channel in ConnectionChannels.members(binding.lease.key.resource)) {
                mux.consumerRetired(channel, binding.lease.key.generation)
            }
        }
        binding.consumersRetired = true
    }
    override fun tick(now: Long) {
        room()
        for (lease in resources.expire(now)) {
            if (lease != null) cancel(lease, Reason.TIMED_OUT)
        }
        settle(now)
    }
    private fun settle(now: Long) {
        for (binding in slots) if (binding != null) settleOne(binding, now)
    }
    private fun settleOne(binding: Binding, now: Long) {
        if (!binding.closing || !binding.nativeRetired) return
        if (!finishChannels(binding)) return
        val lease = binding.lease
        queueAcknowledgement(binding)
        if (!binding.acknowledged || (!connected && !transportGone)) return
        resources.finish(lease)
        slots[lease.key.resource.ordinal] = null
        notice(lease, ConnectionResourceState.RETIRED)
        binding.deferred?.let { offered(it, now) }
    }
    private fun queueAcknowledgement(binding: Binding) {
        if (!connected || binding.ackQueued || !resources.readyToAck(binding.lease)) return
        publications.queue(ResultMessage(binding.lease.key, Outcome.REVOKED), binding.lease)
        binding.ackQueued = true
    }
    private fun finishChannels(binding: Binding): Boolean {
        if (!binding.installed || binding.channelsRetired) return true
        if (connected) {
            val members = ConnectionChannels.members(binding.lease.key.resource)
            if (!members.all { mux.retired(it) }) return false
            if (binding.lease.key.generation < Long.MAX_VALUE) {
                for (channel in members) mux.replaceChannel(channel, binding.lease.key.generation + 1)
            }
        } else if (!transportGone) return false
        resources.channelsRetired(binding.lease)
        binding.channelsRetired = true
        return true
    }
    override fun disconnect() {
        if (!connected) return
        connected = false
        resources.disconnect()
        publications.clear()
        for (entry in slots) {
            val binding = entry ?: continue
            binding.deferred = null
            binding.acknowledged = true
            if (!binding.closing) {
                binding.closing = true
                notice(binding.lease, ConnectionResourceState.CLOSING)
            }
        }
    }
    override fun transportRetired() {
        check(!connected && !transportGone)
        if (writer != null) { mux.finish(false); writer = null }
        transportGone = true
        settle(0)
    }
    override fun nextEvent(): ConnectionSessionEvent? {
        if (events.isNotEmpty()) return events.removeFirst()
        for (distance in readable.indices) {
            val index = (cursor + distance) % readable.size
            val lease = readable[index] ?: continue
            readable[index] = null
            if (current(lease) && channels.readable(index + 1)) {
                cursor = (index + 1) % readable.size
                return ConnectionSessionEvent.Readable(lease, index + 1)
            }
        }
        return null
    }
}
