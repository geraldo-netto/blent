package com.blent.connection

/** Prompt local-resource ownership boundary for connection orchestration. */
internal interface LocalConnectionResources {
    fun advance(now: Long)
    fun display(): LocalDisplayRoute?
    fun stop(now: Long)
    fun retired(): Boolean
    fun quarantined(): Boolean
}

/** Serialized resource controller. Fixed slots and bounded event work; a stuck
 * channel quarantines its resource while the connection keeps serving others.
 * Camera is composed only with an explicit original-resource authority adapter;
 * permission remains pending before any camera native factory is entered.
 * Production capability enablement and hardware capture remain external. */
internal class ConnectionLocalResources(
    private val endpoints: ConnectionEndpoints,
    capacity: Int,
    private val limits: ConnectionRunLimits = ConnectionRunLimits(),
    private val factory: (Long) -> LocalEndpoint = { now -> LocalEndpointOwner(LocalSocketEndpoint(), now, limits) },
    private val cameraAuthority: ConnectionResourceAuthority? = null,
) : LocalConnectionResources, LocalCameraControl {
    private val block = minOf(capacity, ConnectionWire.MAX_PAYLOAD)
    private val slots = arrayOfNulls<LocalResourceOwnership>(4)
    private var stopped = false
    private var previous = 0L
    init { ConnectionWindow.validate(capacity) }
    private fun time(now: Long) {
        require(now >= previous && now <= Long.MAX_VALUE - maxOf(limits.ioTimeoutMs, limits.retirementTimeoutMs))
        previous = now
    }
    override fun advance(now: Long) {
        time(now)
        events(now)
        for (group in slots) group?.advance(now)
    }
    private fun events(now: Long) {
        repeat(32) {
            when (val event = endpoints.nextEvent() ?: return) {
                is ConnectionSessionEvent.Offered -> offered(event, now)
                is ConnectionSessionEvent.State -> state(event, now)
                is ConnectionSessionEvent.Readable -> Unit // Pumps poll fixed channels every tick.
            }
        }
    }
    private fun offered(event: ConnectionSessionEvent.Offered, now: Long) {
        val index = event.lease.key.resource.ordinal
        check(slots[index] == null) { "Local resource slot still owned" }
        val resource = if (stopped) null else owner(event)
        if (stopped || resource == null) {
            endpoints.reject(event.lease, ConnectionManagement.Outcome.UNAVAILABLE)
            endpoints.nativeRetired(event.lease)
            return
        }
        slots[index] = resource
        resource.open(now, factory)
    }
    private fun owner(event: ConnectionSessionEvent.Offered): LocalResourceOwnership? = try {
        when (event.lease.key.resource) {
            ConnectionManagement.Resource.DISPLAY -> if (event.profile == ConnectionManagement.Display)
                LocalResourceGroup(event, endpoints, block, limits) else null
            ConnectionManagement.Resource.CAMERA -> cameraAuthority?.let {
                LocalCameraOwner(event, endpoints, block, limits, it)
            }
            else -> null
        }
    } catch (_: IllegalArgumentException) { null } // Constructors are pure; no native work was exposed.
    private fun state(event: ConnectionSessionEvent.State, now: Long) {
        val index = event.lease.key.resource.ordinal
        val group = slots[index] ?: return // Protocol can reject unsupported offers itself.
        if (group.offer.lease != event.lease) return
        when (event.state) {
            ConnectionResourceState.ADMITTED -> group.admit(now)
            ConnectionResourceState.CLOSING -> group.stop(now)
            ConnectionResourceState.RETIRED -> { check(group.retired()); slots[index] = null }
        }
    }
    override fun display(): LocalDisplayRoute? = (slots[ConnectionManagement.Resource.DISPLAY.ordinal] as? LocalDisplayProjection)?.route()
    private fun cameraControl(): LocalCameraControl? = slots[ConnectionManagement.Resource.CAMERA.ordinal] as? LocalCameraControl
    override fun offered(): LocalCameraOffer? = cameraControl()?.offered()
    override fun pending(): CameraPermissionRequest? = cameraControl()?.pending()
    override fun camera(): LocalCameraRoute? = cameraControl()?.camera()
    override fun choose(lease: ConnectionResourceLease, choice: CameraConsentChoice, now: Long): Boolean {
        time(now)
        return !stopped && cameraControl()?.choose(lease, choice, now) == true
    }
    override fun permissionResult(prompt: CameraPermissionRequest, granted: Boolean, choice: CameraConsentChoice, now: Long): Boolean {
        time(now)
        return !stopped && cameraControl()?.permissionResult(prompt, granted, choice, now) == true
    }
    override fun consumerRetired(lease: ConnectionResourceLease, now: Long): Boolean {
        time(now)
        return cameraControl()?.consumerRetired(lease, now) == true
    }
    override fun stop(now: Long) {
        time(now)
        stopped = true
        for (group in slots) group?.stop(now)
    }
    override fun retired(): Boolean = stopped && slots.all { it == null || it.retired() }
    override fun quarantined(): Boolean = slots.any { it?.quarantined() == true }
}

/** Composition retains transport and local ownership independently. A local
 * resource quarantine only requests process termination after connection stop;
 * it cannot tear down unrelated live media. The outer setup owner checks pairing
 * authority before advance. */
internal class ConnectionLocalRun(
    private val transport: ConnectionRun,
    private val local: LocalConnectionResources,
) : ConnectionRun {
    private var stopping = false
    override fun display(): LocalDisplayRoute? = if (stopping) null else local.display()
    override fun endpoints(): ConnectionEndpoints = transport.endpoints()
    override fun advance(now: Long) {
        transport.advance(now)
        if (transport.failure() != null || transport.retired()) stop(now)
        local.advance(now)
    }
    override fun stop(now: Long) {
        stopping = true
        local.stop(now)
        transport.stop(now)
    }
    override fun failure(): Exception? = transport.failure()
    override fun retired(): Boolean = transport.retired() && local.retired()
    override fun terminationRequired(): Boolean = transport.terminationRequired() || (stopping && local.quarantined())
}
