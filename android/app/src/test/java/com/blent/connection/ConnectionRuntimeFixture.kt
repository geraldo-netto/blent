package com.blent.connection

import java.util.ArrayDeque

internal class RuntimeSetup : ConnectionSetup {
    val ticket = ConnectionTicket(1, ConnectionPeer.parse("ab".repeat(32)))
    val approval = ConnectionApproval(ticket, false)
    val events = ArrayDeque<ConnectionWorkerEvent>()
    var ready: ConnectionSetupReady? = null
    var live = true
    var stopping = false
    var gone = false
    var terminate = false
    var error: Exception? = null
    var ticks = 0
    var stops = 0
    var decided = false
    var revoked: ConnectionPeer? = null
    fun offer(capacity: Int = 4096, selected: ConnectionApproval = approval) {
        ready = ConnectionSetupReady(RunnerIo(), selected, capacity)
    }
    override fun advance() { ticks++ }
    override fun decide(ticket: ConnectionTicket, approved: Boolean, remember: Boolean): Boolean {
        decided = approved && remember
        return ticket == this.ticket
    }
    override fun revoke(peer: ConnectionPeer): Boolean { revoked = peer; if (peer == ticket.peer) live = false; return true }
    override fun current(approval: ConnectionApproval) = live && approval == this.approval
    override fun takeReady() = ready.also { ready = null }
    override fun poll() = events.pollFirst()
    override fun stop() { stopping = true; live = false; stops++ }
    override fun retired() = gone
    override fun failure() = error
    override fun terminationRequired() = terminate
}

internal class RuntimeRun(private val endpoint: ConnectionEndpoints = RunnerSession()) : ConnectionRun {
    val dispatch = ArrayList<Boolean>()
    var stopped = false
    var stoppedAt = -1L
    var gone = false
    var terminate = false
    var error: Exception? = null
    var stopError: Exception? = null
    override fun endpoints() = endpoint
    override fun advance(now: Long) { dispatch.add(stopped) }
    override fun stop(now: Long) {
        stopped = true; stoppedAt = now
        (endpoint as? ConnectionSession)?.disconnect()
        stopError?.let { throw it }
    }
    override fun failure() = error
    override fun retired() = gone
    override fun terminationRequired() = terminate
}

internal class RuntimeRig(val run: RuntimeRun = RuntimeRun()) {
    val setup = RuntimeSetup()
    var now = 0L
    var creates = 0
    var factoryError: Exception? = null
    var clockError: Exception? = null
    val owner = ConnectionRuntimeOwner(setup, ConnectionRunFactory {
        creates++; factoryError?.let { throw it }; run
    }, { clockError?.let { throw it }; now })
    fun start() { setup.offer(); owner.advance() }
    fun lease(ticket: ConnectionTicket = setup.ticket) = ConnectionResourceLease(
        ticket, ConnectionManagement.Key(ConnectionManagement.Resource.DISPLAY, 1, 1), 1)
}
