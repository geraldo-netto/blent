package com.blent.connection

import java.security.MessageDigest
import java.util.ArrayDeque

/** One native authority owner and a separately supervised closer. Pairing and
 * trust never cross threads. The lock protects only bounded control state. */
internal class ConnectionSetupOwner(
    private val backend: ConnectionSetupBackend,
    private val store: (ConnectionPeer) -> PeerTrustStore,
    private val clock: () -> Long,
    private val limits: ConnectionSetupLimits = ConnectionSetupLimits(),
    private val wake: () -> Unit = {},
    private val threads: (String, Runnable) -> Thread = { name, body -> Thread(body, name).apply { isDaemon = true } },
) : ConnectionSetup {
    private class Decision(val ticket: ConnectionTicket, val approved: Boolean, val remember: Boolean)
    private val lock = Object()
    private val events = ArrayDeque<ConnectionWorkerEvent>()
    private val revokes = ArrayDeque<ConnectionPeer>()
    private var decision: Decision? = null
    private var candidate: ConnectionTicket? = null
    private var waiting = false
    private var stopped = false
    private var terminal = false
    private var error: Exception? = null
    private var closeError: Exception? = null
    private var approval: ConnectionApproval? = null
    private var ready: ConnectionSetupReady? = null
    private var ownedIo: ConnectionIo? = null
    private var lastTime = checkedTime()
    private var deadline = until(lastTime, limits.nativeTimeoutMs)
    private var stopDeadline = Long.MAX_VALUE
    private var worker: Thread? = null
    private var closer: Thread? = null
    private var closeFinished = false
    private var startFinished = false

    init { start() }

    private fun checkedTime(): Long = clock().also { require(it >= 0) }
    private fun until(now: Long, duration: Long): Long {
        require(now <= Long.MAX_VALUE - duration)
        return now + duration
    }
    private fun now(): Long = synchronized(lock) {
        checkedTime().also { require(it >= lastTime); lastTime = it }
    }

    private fun start() {
        try {
            closer = threads("blent-setup-close", Runnable { closeNative() })
            checkNotNull(closer).start()
            worker = threads("blent-setup-authority", Runnable { runNative() })
            checkNotNull(worker).start()
        } catch (failure: Exception) { fail(failure) }
        finally { synchronized(lock) { startFinished = true }; signal() }
    }

    private fun signal() {
        try { wake() } catch (failure: Exception) { synchronized(lock) { failLocked(failure) } }
    }

    private fun event(value: ConnectionWorkerEvent) {
        synchronized(lock) {
            if (events.size == 32) { failLocked(IllegalStateException("Setup event backlog")); return }
            events.addLast(value)
        }
        signal()
    }

    private fun stopLocked() {
        if (stopped) return
        stopped = true; approval = null; ready = null
        val time = stopTime()
        stopDeadline = time + minOf(limits.retirementTimeoutMs, Long.MAX_VALUE - time)
        lock.notifyAll()
    }

    private fun stopTime(): Long = try { now() }
        catch (failure: Exception) { if (error == null) error = failure; lastTime }

    private fun failLocked(failure: Exception) { if (error == null) error = failure; stopLocked() }
    private fun fail(failure: Exception) { synchronized(lock) { failLocked(failure) }; signal() }
    override fun stop() { synchronized(lock) { stopLocked() }; signal() }

    override fun advance() {
        try {
            val time = now()
            synchronized(lock) {
                if (!stopped && time >= deadline) failLocked(IllegalStateException("Setup deadline expired"))
            }
        } catch (failure: Exception) { fail(failure) }
    }

    override fun decide(ticket: ConnectionTicket, approved: Boolean, remember: Boolean): Boolean {
        synchronized(lock) {
            if (stopped || !waiting || candidate != ticket || decision != null) return false
            decision = Decision(ticket, approved, remember)
            if (!approved) stopLocked()
            lock.notifyAll()
        }
        signal(); return true
    }

    override fun revoke(peer: ConnectionPeer): Boolean {
        synchronized(lock) {
            if (stopped || terminal || revokes.size == 32) return false
            if (revokes.contains(peer)) return false
            revokes.addLast(peer)
            if (candidate?.peer == peer) stopLocked()
            lock.notifyAll()
        }
        signal(); return true
    }

    override fun current(approval: ConnectionApproval): Boolean = synchronized(lock) { !stopped && this.approval == approval }
    override fun takeReady(): ConnectionSetupReady? = synchronized(lock) { ready.also { ready = null } }
    override fun poll(): ConnectionWorkerEvent? = synchronized(lock) { events.pollFirst() }
    override fun failure(): Exception? = synchronized(lock) { error ?: closeError }
    override fun retired(): Boolean = synchronized(lock) {
        startFinished && stopped && closeFinished && worker?.isAlive != true && closer?.isAlive != true &&
            ownedIo?.retired() != false && backend.retired()
    }
    override fun terminationRequired(): Boolean {
        val overdue = synchronized(lock) { stopped && lastTime >= stopDeadline }
        return synchronized(lock) { closeError != null } || (overdue && !retired())
    }

    private fun live() { synchronized(lock) {
        check(!stopped) { "Setup cancelled" }
        check(now() < deadline) { "Setup deadline expired" }
    } }
    private fun stage(duration: Long, phase: ConnectionWorkerPhase) {
        stageUntil(until(now(), duration), phase)
    }
    private fun stageUntil(end: Long, phase: ConnectionWorkerPhase) {
        synchronized(lock) { live(); deadline = end }
        event(ConnectionWorkerEvent.State(phase))
    }
    private fun peer(identity: ConnectionIdentity): ConnectionPeer {
        val digest = MessageDigest.getInstance("SHA-256").digest(identity.certificate())
        return ConnectionPeer.parse(digest.joinToString("") { "%02x".format(it.toInt() and 255) })
    }

    private fun runNative() {
        var trust: ConnectionTrust? = null
        val pairing = ConnectionPairing()
        try {
            stage(limits.nativeTimeoutMs, ConnectionWorkerPhase.OPENING)
            val local = peer(backend.localIdentity())
            live()
            trust = ConnectionTrust(store(local), local, limits.trustedPeers)
            establish(trust, pairing, local)
            service(trust, pairing)
        } catch (failure: Exception) { fail(failure) }
        finally { finish(trust, pairing) }
    }

    private fun establish(trust: ConnectionTrust, pairing: ConnectionPairing, local: ConnectionPeer) {
        live()
        val identity = backend.discover(limits.nativeTimeoutMs)
        val peer = peer(identity)
        val begun = now()
        val ticket = trust.begin(pairing, peer, begun, limits.approvalTimeoutMs)
        synchronized(lock) {
            candidate = ticket
            if (revokes.contains(peer)) stopLocked()
        }
        stageUntil(until(begun, limits.approvalTimeoutMs), ConnectionWorkerPhase.WAITING_APPROVAL)
        if (pairing.phase == ConnectionPairing.Phase.PENDING) awaitApproval(pairing, ticket, local)
        else event(ConnectionWorkerEvent.Peer(ticket, local, trust.trusted(peer)))
        live()
        val proven = authenticate(identity)
        live()
        stage(limits.nativeTimeoutMs, ConnectionWorkerPhase.OPENING)
        val approved = trust.authenticated(pairing, ticket, proven, now())
        if (approved.remember) event(ConnectionWorkerEvent.Trust(approved.ticket.peer, true))
        live()
        stage(limits.nativeTimeoutMs, ConnectionWorkerPhase.OPENING)
        val capacity = backend.negotiate(limits.capacity, limits.nativeTimeoutMs)
        ConnectionWindow.accepted(limits.capacity, capacity)
        install(approved, capacity)
    }

    private fun authenticate(identity: ConnectionIdentity): ConnectionPeer {
        val approvalDeadline = synchronized(lock) { deadline }
        val remaining = approvalDeadline - now()
        require(remaining > 0)
        backend.ready(identity, remaining)
        live()
        val time = now()
        require(time < approvalDeadline) { "Approval expired during readiness" }
        val tlsDeadline = minOf(approvalDeadline, until(time, limits.nativeTimeoutMs))
        stageUntil(tlsDeadline, ConnectionWorkerPhase.OPENING)
        return backend.authenticate(identity)
    }

    private fun awaitApproval(pairing: ConnectionPairing, ticket: ConnectionTicket, local: ConnectionPeer) {
        synchronized(lock) { waiting = true }
        event(ConnectionWorkerEvent.Pairing(ticket, local))
        val selected = synchronized(lock) {
            while (!stopped && decision == null) lock.wait()
            waiting = false
            checkNotNull(decision)
        }
        check(selected.approved)
        pairing.approve(selected.ticket, selected.remember, now())
    }

    private fun install(approved: ConnectionApproval, capacity: Int) {
        val io = backend.io(::signal)
        synchronized(lock) {
            ownedIo = io
            if (stopped) { io.stop(); return }
            live()
            approval = approved
            ready = ConnectionSetupReady(io, approved, capacity)
            deadline = Long.MAX_VALUE
            event(ConnectionWorkerEvent.State(ConnectionWorkerPhase.AUTHENTICATED))
        }
        signal()
    }

    private fun service(trust: ConnectionTrust, pairing: ConnectionPairing) {
        while (true) {
            val revoke = synchronized(lock) {
                while (!stopped && revokes.isEmpty()) lock.wait()
                revokes.pollFirst()
            }
            if (revoke == null) return
            persistRevoke(trust, pairing, revoke)
        }
    }

    private fun persistRevoke(trust: ConnectionTrust, pairing: ConnectionPairing, peer: ConnectionPeer) {
        val bounded = mutationDeadline()
        trust.revoke(pairing, peer)
        event(ConnectionWorkerEvent.Trust(peer, false))
        synchronized(lock) {
            if (bounded && !stopped) { live(); deadline = Long.MAX_VALUE }
        }
    }

    private fun mutationDeadline(): Boolean = synchronized(lock) {
        if (stopped) return false
        live()
        deadline = until(now(), limits.nativeTimeoutMs)
        true
    }

    private fun finish(trust: ConnectionTrust?, pairing: ConnectionPairing) {
        stop()
        try {
            if (trust != null) drainRevokes(trust, pairing)
        } catch (failure: Exception) { fail(failure) }
        finally {
            synchronized(lock) { terminal = true }
            candidate?.let { pairing.cancel(it) }
            try { trust?.close() } catch (failure: Exception) { fail(failure) }
            event(ConnectionWorkerEvent.Ended(failure()?.javaClass?.simpleName))
        }
    }

    private fun drainRevokes(trust: ConnectionTrust, pairing: ConnectionPairing) {
        while (true) {
            val peer = synchronized(lock) { revokes.pollFirst() } ?: return
            persistRevoke(trust, pairing, peer)
        }
    }

    private fun closeNative() {
        try {
            synchronized(lock) { while (!stopped) lock.wait() }
            synchronized(lock) { ownedIo }?.stop()
            backend.close()
        } catch (failure: Exception) { synchronized(lock) { closeError = failure } }
        finally { synchronized(lock) { closeFinished = true }; signal() }
    }
}
