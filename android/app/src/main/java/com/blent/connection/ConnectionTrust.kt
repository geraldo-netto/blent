package com.blent.connection

/** Exclusive storage ownership lasts until close. Failed publication must leave
 * an explicit interrupted-operation marker; no implicit repair is permitted. */
internal interface PeerTrustStore : AutoCloseable {
    fun load(): ByteArray?
    fun replace(record: ByteArray)
}

/** One serialized owner must cover this object AND all supplied pairing policies.
 * Stored trust only suppresses a prompt; current TLS proof remains mandatory.
 * The broker must revoke every live pairing for the selected peer. */
internal class ConnectionTrust(
    private val store: PeerTrustStore,
    private val local: ConnectionPeer,
    private val capacity: Int = 64,
) : AutoCloseable {
    private var revision = 0L
    private val peers = sortedSetOf<String>()
    private var poisoned = false
    private var closed = false

    init {
        try {
            require(capacity in 1..1024)
            restore()
        }
        catch (failure: Exception) {
            try { store.close() } catch (cleanup: Exception) { failure.addSuppressed(cleanup) }
            throw failure
        }
    }

    private fun restore() {
        val record = store.load() ?: return
        val snapshot = ConnectionTrustCodec.decode(record)
        require(snapshot.local == local && snapshot.peers.size <= capacity)
        revision = snapshot.revision
        snapshot.peers.forEach { peers.add(it.fingerprint) }
    }

    fun trusted(peer: ConnectionPeer): Boolean = !closed && !poisoned && peers.contains(peer.fingerprint)

    /** Storage failure cannot silently become an unremembered pairing prompt. */
    fun begin(pairing: ConnectionPairing, peer: ConnectionPeer, now: Long, timeout: Long): ConnectionTicket {
        check(!closed && !poisoned)
        return pairing.begin(peer, trusted(peer), now, timeout)
    }

    /** Invoke only for the stock TLS adapter's verified peer event. A failed
     * remembered admission cannot leave its newly authenticated lease alive. */
    fun authenticated(
        pairing: ConnectionPairing,
        ticket: ConnectionTicket,
        proven: ConnectionPeer,
        now: Long,
    ): ConnectionApproval {
        try {
            check(!closed && !poisoned)
            val lease = pairing.authenticated(ticket, proven, now)
            if (lease.remember) remember(pairing, lease)
            return lease
        } catch (failure: Exception) {
            pairing.cancel(ticket)
            throw failure
        }
    }

    fun remember(pairing: ConnectionPairing, lease: ConnectionApproval): Boolean {
        check(!closed && !poisoned)
        check(lease.remember && pairing.current(lease))
        val peer = lease.ticket.peer.fingerprint
        if (peers.contains(peer)) return false
        check(peers.size < capacity)
        val updated = peers.toSortedSet().apply { add(peer) }
        persist(updated)
        peers.add(peer)
        return true
    }

    fun revoke(pairing: ConnectionPairing, peer: ConnectionPeer): Boolean {
        pairing.revoke(peer)
        val removed = peers.remove(peer.fingerprint)
        check(!closed && !poisoned)
        if (!removed) return false
        persist(peers)
        return true
    }

    private fun persist(updated: Set<String>) {
        poisoned = true
        check(revision < Long.MAX_VALUE) { "Trust revision exhausted" }
        val next = TrustSnapshot(local, revision + 1, updated.map(ConnectionPeer::parse))
        store.replace(ConnectionTrustCodec.encode(next))
        revision = next.revision
        poisoned = false
    }

    override fun close() {
        if (closed) return
        closed = true
        peers.clear()
        store.close()
    }
}
