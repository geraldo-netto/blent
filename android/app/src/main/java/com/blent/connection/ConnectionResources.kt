package com.blent.connection

import com.blent.connection.ConnectionResourceLease as Lease
import com.blent.connection.ConnectionManagement.Key
import com.blent.connection.ConnectionManagement.Outcome
import com.blent.connection.ConnectionManagement.Resource

/** T851: four resource slots, owned by one authenticated connection event loop.
 * Reserve bounded management output before an outgoing transition; preserve FIFO
 * through cancellation. Commit/revokeWritten mean whole-record write completion.
 * Queue peer events behind those barriers. Failed writes disconnect; never undo. */
internal class ConnectionResources(private val role: Role, private val connection: ConnectionTicket) {
    enum class Role { HOST, TABLET }
    enum class Install { ACTIVATE, DRAIN }
    private enum class Stage { OFFERED, PREPARED, COMMITTING, COMMITTED, ADMITTED }
    private class Slot(val lease: Lease, val deadline: Long) {
        var stage = Stage.OFFERED
        var closing = false
        var permissionReported = false
        var rejected = false
        var peerRevoke = false
        var revokeWritten = false
        var settled = false
        var installed = false
        var nativeRetired = false
        var channelsRetired = true
        fun live(now: Long) { check(!closing && now >= 0 && now < deadline) }
    }
    private var connected = true
    private var request = 0L
    private val lastRequests = LongArray(4)
    private val generations = LongArray(4) { 1 }
    private val slots = arrayOfNulls<Slot>(4)
    init { require(connection.epoch > 0) }

    fun begin(resource: Resource, owner: Long, now: Long, timeout: Long): Lease {
        check(role == Role.HOST && request < Long.MAX_VALUE)
        val key = Key(resource, request + 1, generations[resource.ordinal])
        return insert(key, owner, now, timeout).also { request = key.request }
    }
    fun offer(key: Key, owner: Long, now: Long, timeout: Long): Lease {
        check(role == Role.TABLET)
        return insert(key, owner, now, timeout)
    }
    private fun insert(key: Key, owner: Long, now: Long, timeout: Long): Lease {
        require(key.request > 0 && key.generation > 0)
        check(connected && owner > 0)
        require(now >= 0 && timeout in 1..600_000 && now <= Long.MAX_VALUE - timeout)
        val index = key.resource.ordinal
        check(slots[index] == null)
        check(key.request > lastRequests[index] && key.generation == generations[index])
        val lease = Lease(connection, key, owner)
        slots[index] = Slot(lease, now + timeout)
        lastRequests[index] = key.request
        return lease
    }
    private fun slot(lease: Lease): Slot {
        val slot = checkNotNull(slots[lease.key.resource.ordinal])
        check(slot.lease == lease)
        return slot
    }
    private fun online(lease: Lease): Slot {
        check(connected)
        val slot = slot(lease)
        check(!slot.settled)
        return slot
    }
    fun current(lease: Lease): Boolean {
        val slot = slots[lease.key.resource.ordinal] ?: return false
        return connected && slot.lease == lease && !slot.closing && slot.stage == Stage.ADMITTED
    }
    fun revoke(lease: Lease): Boolean {
        val slot = slot(lease)
        val changed = !slot.closing
        slot.closing = true
        return changed
    }
    fun expire(now: Long): Array<Lease?> {
        require(now >= 0)
        val expired = arrayOfNulls<Lease>(4)
        for ((index, slot) in slots.withIndex()) {
            if (slot != null && !slot.closing && slot.stage != Stage.ADMITTED && now >= slot.deadline) {
                slot.closing = true
                expired[index] = slot.lease
            }
        }
        return expired
    }
    /** Link loss waives peer settlement only, never native/borrow retirement. */
    fun disconnect() {
        connected = false
        for (slot in slots.filterNotNull()) { slot.closing = true; slot.settled = true }
    }
    fun nativeRetired(lease: Lease) {
        val slot = slot(lease)
        check(slot.closing && !slot.nativeRetired)
        slot.nativeRetired = true
    }
    /** Both display/control channels must retire for DISPLAY. After link loss,
     * actual consumer/writer exit replaces unavailable peer terminal evidence. */
    fun channelsRetired(lease: Lease) {
        val slot = slot(lease)
        check(slot.closing && slot.installed && !slot.channelsRetired)
        slot.channelsRetired = true
    }
    fun finish(lease: Lease) {
        val slot = slot(lease)
        check(slot.closing && slot.settled && slot.nativeRetired && slot.channelsRetired)
        val index = lease.key.resource.ordinal
        if (slot.installed) {
            // Zero is an exhausted sentinel rejected by insert; never overflow.
            generations[index] = if (generations[index] == Long.MAX_VALUE) 0 else generations[index] + 1
        }
        slots[index] = null
    }
    /** Tablet commits Prepared to bounded FIFO after consent/preparation; host
     * calls on receipt. Neither side grants media authority here. */
    fun prepared(lease: Lease, now: Long): Boolean {
        val slot = online(lease)
        check(slot.stage == Stage.OFFERED && !slot.rejected)
        if (role == Role.TABLET) slot.live(now)
        require(now >= 0)
        slot.closing = slot.closing || now >= slot.deadline
        slot.stage = Stage.PREPARED
        return !slot.closing
    }
    fun permissionRequired(lease: Lease, now: Long): Boolean {
        val slot = online(lease)
        check(slot.stage == Stage.OFFERED && !slot.permissionReported && !slot.rejected)
        if (role == Role.TABLET) slot.live(now)
        require(now >= 0)
        slot.permissionReported = true
        slot.closing = slot.closing || now >= slot.deadline
        return !slot.closing
    }
    /** REVOKED is reserved for final ACK. Failure after Prepared uses Revoke. */
    fun reject(lease: Lease, outcome: Outcome) {
        require(outcome in listOf(Outcome.UNAVAILABLE, Outcome.PERMISSION_DENIED, Outcome.BUSY, Outcome.FAILED))
        val slot = online(lease)
        check(slot.stage == Stage.OFFERED && !slot.rejected)
        check(role == Role.HOST || !slot.closing)
        slot.rejected = true
        slot.closing = true
    }
    fun startCommit(lease: Lease, now: Long) {
        check(role == Role.HOST)
        val slot = online(lease)
        slot.live(now)
        check(slot.stage == Stage.PREPARED)
        slot.stage = Stage.COMMITTING
    }
    /** Host: entire Commit written, borrow retired, before peer events. Tablet:
     * complete Commit received, before next envelope. DRAIN installs/closes the
     * inactive generation without granting credit or media authority. */
    fun commit(lease: Lease, now: Long): Install {
        val expected = if (role == Role.HOST) Stage.COMMITTING else Stage.PREPARED
        val slot = online(lease)
        require(now >= 0)
        check(slot.stage == expected && !(expected == Stage.PREPARED && slot.peerRevoke))
        slot.closing = slot.closing || now >= slot.deadline
        slot.stage = Stage.COMMITTED
        slot.installed = true
        slot.channelsRetired = false
        return if (slot.closing) Install.DRAIN else Install.ACTIVATE
    }
    /** Admission grants a current lease, not proof of successful media playback. */
    fun admitted(lease: Lease, now: Long): Boolean {
        val slot = online(lease)
        check(slot.stage == Stage.COMMITTED)
        if (role == Role.TABLET) slot.live(now)
        require(now >= 0)
        slot.closing = slot.closing || now >= slot.deadline
        slot.stage = Stage.ADMITTED
        return !slot.closing
    }
    fun peerRevoke(lease: Lease) {
        val slot = online(lease)
        check(!slot.peerRevoke)
        slot.peerRevoke = true
        slot.closing = true
    }
    fun revokeWritten(lease: Lease) {
        check(role == Role.HOST)
        val slot = online(lease)
        check(slot.closing && slot.stage != Stage.COMMITTING && !slot.revokeWritten)
        slot.revokeWritten = true
    }
    /** Tablet checks readyToAck before queueing ACK, then calls this only after
     * full ACK write. Host calls on receipt behind its Revoke write barrier. */
    fun acknowledge(lease: Lease) {
        check(readyToAck(lease))
        online(lease).settled = true
    }
    fun readyToAck(lease: Lease): Boolean {
        val slot = online(lease)
        if (!slot.closing) return false
        return when (role) {
            Role.HOST -> slot.revokeWritten
            Role.TABLET -> slot.peerRevoke && slot.nativeRetired && slot.channelsRetired
        }
    }
}
