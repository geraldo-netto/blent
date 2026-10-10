package com.blent.connection

/** SHA-256 of exact certificate DER, computed by the stock TLS boundary. */
internal data class ConnectionPeer private constructor(val fingerprint: String) {
    init { require(fingerprint.length == 64 && fingerprint.all { it in '0'..'9' || it in 'a'..'f' }) }
    companion object { fun parse(value: String): ConnectionPeer = ConnectionPeer(value) }
}

/** Independent single-owner authority policy; no native I/O, crypto or UI work.
 * TLS proof is a trusted adapter event, never an incoming management command. */
internal class ConnectionPairing {
    data class Ticket(val epoch: Long, val peer: ConnectionPeer)
    data class Lease(val ticket: Ticket, val remember: Boolean)
    enum class Phase { IDLE, PENDING, APPROVED, AUTHENTICATED, CLOSING }
    var phase = Phase.IDLE; private set
    private var epoch = 0L
    private var ticket: Ticket? = null
    private var deadline = 0L
    private var remember = false

    fun begin(peer: ConnectionPeer, trusted: Boolean, now: Long, timeout: Long): Ticket {
        check(phase == Phase.IDLE)
        require(now >= 0 && timeout in 1..600_000 && now <= Long.MAX_VALUE - timeout)
        check(epoch < Long.MAX_VALUE)
        val next = Ticket(++epoch, peer)
        ticket = next; deadline = now + timeout; remember = false
        phase = if (trusted) Phase.APPROVED else Phase.PENDING
        return next
    }

    fun approve(candidate: Ticket, persist: Boolean, now: Long) {
        check(ticket == candidate && phase == Phase.PENDING)
        require(now >= 0 && now < deadline)
        remember = persist; phase = Phase.APPROVED
    }

    /** Called only after verified TLS. Caller rechecks current() before storing
     * trust or installing endpoints; separate Android media consent still applies. */
    fun authenticated(candidate: Ticket, proven: ConnectionPeer, now: Long): Lease {
        check(ticket == candidate && phase == Phase.APPROVED)
        require(now >= 0 && now < deadline && proven == candidate.peer)
        phase = Phase.AUTHENTICATED
        return Lease(candidate, remember)
    }

    fun current(lease: Lease): Boolean = phase == Phase.AUTHENTICATED && ticket == lease.ticket && remember == lease.remember

    fun cancel(candidate: Ticket): Boolean {
        if (ticket != candidate || phase == Phase.IDLE) return false
        phase = Phase.CLOSING
        return true
    }

    fun revoke(peer: ConnectionPeer): Boolean {
        val selected = ticket ?: return false
        if (selected.peer != peer) return false
        return cancel(selected)
    }

    fun expire(now: Long): Boolean {
        require(now >= 0)
        if (phase != Phase.PENDING && phase != Phase.APPROVED) return false
        if (now < deadline) return false
        phase = Phase.CLOSING
        return true
    }

    /** Logical cancellation or a kill request does not establish retirement. */
    fun retired(candidate: Ticket) {
        check(ticket == candidate && phase == Phase.CLOSING)
        phase = Phase.IDLE; ticket = null; remember = false
    }
}
