package com.blent.connection

/** Portable T888 supervision; never opens native resources or persists trust. */
internal class ConnectionRuntimeOwner(
    private val setup: ConnectionSetup,
    private val factory: ConnectionRunFactory,
    private val clock: () -> Long,
) : ConnectionRuntime, LocalCameraProjection, LocalAudioProjection {
    private var runner: ConnectionRun? = null
    private var approval: ConnectionApproval? = null
    private var attempted = false
    private var stopped = false
    private var uncertain = false
    private var previous = 0L
    private var error: Exception? = null
    private val guarded = RuntimeEndpoints(this)

    private fun time(): Long = try {
        clock().also { require(it in previous..Long.MAX_VALUE - 60000); previous = it }
    } catch (failure: Exception) { fail(failure); previous }

    private fun fail(failure: Exception) {
        if (error == null) error = failure
        stopAt(previous)
    }
    private fun stopAt(now: Long) {
        if (stopped) return
        stopped = true
        try { runner?.stop(now) } catch (failure: Exception) { uncertain = true; if (error == null) error = failure }
        try { setup.stop() } catch (failure: Exception) { uncertain = true; if (error == null) error = failure }
    }
    override fun advance() {
        val now = time()
        try {
            setup.advance()
            setup.failure()?.let { fail(it) }
            if (setup.retired() || setup.terminationRequired()) stopAt(now)
            install()
            live()
            runner?.advance(now)
            runner?.failure()?.let { fail(it) }
            if (runner?.retired() == true) stopAt(now)
        } catch (failure: Exception) { fail(failure) }
    }
    private fun install() {
        if (stopped || attempted) return
        val ready = setup.takeReady() ?: return
        ConnectionWindow.validate(ready.capacity)
        require(ready.approval.ticket.epoch > 0)
        if (!setup.current(ready.approval)) { stopAt(previous); return }
        attempted = true
        approval = ready.approval
        // Set uncertainty before entering user construction; only a returned
        // retained owner establishes where actual retirement can be observed.
        uncertain = true
        runner = factory.create(ready)
        uncertain = false
    }
    private fun live(): Boolean {
        if (stopped) return false
        val selected = approval ?: return false
        if (setup.current(selected)) return true
        stopAt(previous)
        return false
    }
    internal fun admission(lease: ConnectionResourceLease): Boolean {
        time()
        return live() && approval?.ticket == lease.connection
    }
    internal fun endpointTime(now: Long): Long {
        val current = time()
        require(now in 0..current) { "Endpoint timestamp is outside the runtime clock" }
        return current
    }
    internal fun retained(lease: ConnectionResourceLease): ConnectionEndpoints {
        require(approval?.ticket == lease.connection) { "Foreign connection lease" }
        return checkNotNull(raw()) { "No resource session installed" }
    }
    internal fun raw(): ConnectionEndpoints? = runner?.endpoints()
    internal fun endpointEvent(): ConnectionSessionEvent? {
        time()
        val allowed = live()
        val endpoints = raw() ?: return null
        repeat(36) {
            val event = endpoints.nextEvent() ?: return null
            if (eventLease(event).connection == approval?.ticket && (allowed || terminal(event))) return event
        }
        return null
    }
    private fun terminal(event: ConnectionSessionEvent): Boolean =
        event is ConnectionSessionEvent.State && event.state != ConnectionResourceState.ADMITTED

    private fun eventLease(event: ConnectionSessionEvent): ConnectionResourceLease = when (event) {
        is ConnectionSessionEvent.Offered -> event.lease
        is ConnectionSessionEvent.Readable -> event.lease
        is ConnectionSessionEvent.State -> event.lease
    }
    override fun command(command: ConnectionWorkerCommand): Boolean {
        time()
        val accepted = when (command) {
            is ConnectionWorkerCommand.Decide -> !stopped && setup.decide(command.ticket, command.approved, command.remember)
            is ConnectionWorkerCommand.Revoke -> setup.revoke(command.peer)
            is ConnectionWorkerCommand.Camera -> cameraCommand(command)
            is ConnectionWorkerCommand.Audio -> audioCommand(command)
        }
        live()
        return accepted
    }
    private fun cameraCommand(value: ConnectionWorkerCommand.Camera): Boolean {
        if (approval?.ticket != value.lease.connection) return false
        val control = runner as? LocalCameraControl ?: return false
        return when (value) {
            is ConnectionWorkerCommand.CameraChoice -> live() && control.choose(value.lease, value.choice, previous)
            is ConnectionWorkerCommand.CameraPermission -> live() && permission(control, value)
            // Cleanup belongs to the retained original owner after logical stop.
            is ConnectionWorkerCommand.CameraConsumerRetired -> control.consumerRetired(value.lease, previous)
        }
    }
    private fun audioCommand(value: ConnectionWorkerCommand.Audio): Boolean {
        if (approval?.ticket != value.lease.connection) return false
        val control = runner as? LocalAudioControl ?: return false
        return when (value) {
            is ConnectionWorkerCommand.AudioChoice -> live() && control.chooseAudio(value.lease, value.choice, previous)
            is ConnectionWorkerCommand.AudioConsumerRetired -> control.audioConsumerRetired(value.lease, previous)
        }
    }
    override fun audioOffered(resource: ConnectionManagement.Resource): LocalAudioOffer? {
        time()
        if (!live()) return null
        return (runner as? LocalAudioProjection)?.audioOffered(resource)?.takeIf { it.lease.key.resource == resource && it.lease.connection == approval?.ticket }
    }
    override fun audio(resource: ConnectionManagement.Resource): LocalAudioRoute? {
        time()
        if (!live()) return null
        val selected = (runner as? LocalAudioProjection)?.audio(resource) ?: return null
        if (selected.lease.key.resource != resource || selected.lease.connection != approval?.ticket) return null
        return selected.takeIf { raw()?.current(it.lease) == true }
    }
    private fun permission(control: LocalCameraControl, value: ConnectionWorkerCommand.CameraPermission): Boolean {
        val prompt = control.pending() ?: return false
        if (prompt.lease != value.lease) return false
        return control.permissionResult(prompt, value.granted, value.choice, previous)
    }
    override fun offered(): LocalCameraOffer? {
        time()
        if (!live()) return null
        return (runner as? LocalCameraProjection)?.offered()?.takeIf { it.lease.connection == approval?.ticket }
    }
    override fun camera(): LocalCameraRoute? {
        time()
        if (!live()) return null
        val selected = (runner as? LocalCameraProjection)?.camera() ?: return null
        if (selected.lease.connection != approval?.ticket) return null
        return selected.takeIf { raw()?.current(it.lease) == true }
    }
    override fun poll(): ConnectionWorkerEvent? {
        time(); live()
        repeat(32) {
            val event = setup.poll() ?: return null
            if (event is ConnectionWorkerEvent.Ended) stopAt(previous)
            if (visible(event)) return event
        }
        return null
    }
    private fun visible(event: ConnectionWorkerEvent): Boolean = when (event) {
        is ConnectionWorkerEvent.Pairing, is ConnectionWorkerEvent.Peer -> !stopped
        is ConnectionWorkerEvent.State -> !stopped || event.phase == ConnectionWorkerPhase.CLOSING
        is ConnectionWorkerEvent.Trust, is ConnectionWorkerEvent.Ended -> true
        is ConnectionWorkerEvent.Media -> false // Only guarded runner snapshots publish media.
    }
    override fun display(): LocalDisplayRoute? {
        time()
        if (!live()) return null
        val selected = runner?.display() ?: return null
        if (selected.lease.connection != approval?.ticket) return null
        return selected.takeIf { raw()?.current(it.lease) == true }
    }
    override fun endpoints(): ConnectionEndpoints = guarded
    override fun stop() { stopAt(time()) }
    override fun failure(): Exception? = error ?: setup.failure() ?: runner?.failure()
    override fun retired(): Boolean = stopped && !uncertain && setup.retired() && (runner?.retired() != false)
    override fun terminationRequired(): Boolean = uncertain || setup.terminationRequired() || runner?.terminationRequired() == true
}
