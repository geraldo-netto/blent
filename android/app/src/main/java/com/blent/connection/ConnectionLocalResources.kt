package com.blent.connection

import com.blent.MediaLocalConsent

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
    factory: ((Long) -> LocalEndpoint)? = null,
    private val cameraAuthority: ConnectionResourceAuthority? = null,
    private val audioAuthority: ConnectionResourceAuthority? = null,
    private val microphoneAuthority: ConnectionResourceAuthority? = null,
    wake: () -> Unit = {},
) : LocalConnectionResources, LocalCameraControl, LocalAudioControl, LocalAudioPermissionControl {
    private val factory = factory ?: { now: Long ->
        LocalEndpointOwner(LocalSocketEndpoint(wake), now, limits, wake)
    }
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
            ConnectionManagement.Resource.SPEAKERS -> audioAuthority?.let {
                LocalSpeakerOwner(event, endpoints, block, limits, it)
            }
            ConnectionManagement.Resource.MICROPHONE -> microphoneAuthority?.let {
                LocalAudioOwner(event, endpoints, block, limits, it)
            }
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
    private fun audioControl(resource: ConnectionManagement.Resource): LocalAudioControl? = slots[resource.ordinal] as? LocalAudioControl
    override fun audioOffered(resource: ConnectionManagement.Resource): LocalAudioOffer? = audioControl(resource)?.audioOffered(resource)
    override fun audio(resource: ConnectionManagement.Resource): LocalAudioRoute? = audioControl(resource)?.audio(resource)
    override fun chooseAudio(lease: ConnectionResourceLease, choice: MediaLocalConsent, now: Long): Boolean {
        time(now)
        return !stopped && audioControl(lease.key.resource)?.chooseAudio(lease, choice, now) == true
    }
    override fun audioConsumerRetired(lease: ConnectionResourceLease, now: Long): Boolean {
        time(now)
        return audioControl(lease.key.resource)?.audioConsumerRetired(lease, now) == true
    }
    private fun audioPermissionControl(): LocalAudioPermissionControl? =
        slots[ConnectionManagement.Resource.MICROPHONE.ordinal] as? LocalAudioPermissionControl
    override fun pendingAudio(): AudioPermissionRequest? = audioPermissionControl()?.pendingAudio()
    override fun audioPermissionResult(prompt: AudioPermissionRequest, granted: Boolean, choice: MediaLocalConsent, now: Long): Boolean {
        time(now)
        return !stopped && audioPermissionControl()?.audioPermissionResult(prompt, granted, choice, now) == true
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
) : ConnectionRun, LocalCameraControl, LocalAudioControl, LocalAudioPermissionControl {
    private var stopping = false
    override fun display(): LocalDisplayRoute? = if (stopping) null else local.display()
    private fun cameraControl(): LocalCameraControl? = local as? LocalCameraControl
    override fun offered(): LocalCameraOffer? = if (stopping) null else cameraControl()?.offered()
    override fun camera(): LocalCameraRoute? = if (stopping) null else cameraControl()?.camera()
    override fun pending(): CameraPermissionRequest? = if (stopping) null else cameraControl()?.pending()
    override fun choose(lease: ConnectionResourceLease, choice: CameraConsentChoice, now: Long): Boolean =
        !stopping && cameraControl()?.choose(lease, choice, now) == true
    override fun permissionResult(prompt: CameraPermissionRequest, granted: Boolean, choice: CameraConsentChoice, now: Long): Boolean =
        !stopping && cameraControl()?.permissionResult(prompt, granted, choice, now) == true
    override fun consumerRetired(lease: ConnectionResourceLease, now: Long): Boolean =
        cameraControl()?.consumerRetired(lease, now) == true
    private fun audioControl(): LocalAudioControl? = local as? LocalAudioControl
    override fun audioOffered(resource: ConnectionManagement.Resource): LocalAudioOffer? =
        if (stopping) null else audioControl()?.audioOffered(resource)
    override fun audio(resource: ConnectionManagement.Resource): LocalAudioRoute? =
        if (stopping) null else audioControl()?.audio(resource)
    override fun chooseAudio(lease: ConnectionResourceLease, choice: MediaLocalConsent, now: Long): Boolean =
        !stopping && audioControl()?.chooseAudio(lease, choice, now) == true
    override fun audioConsumerRetired(lease: ConnectionResourceLease, now: Long): Boolean =
        audioControl()?.audioConsumerRetired(lease, now) == true
    private fun audioPermissionControl(): LocalAudioPermissionControl? = local as? LocalAudioPermissionControl
    override fun pendingAudio(): AudioPermissionRequest? = if (stopping) null else audioPermissionControl()?.pendingAudio()
    override fun audioPermissionResult(prompt: AudioPermissionRequest, granted: Boolean, choice: MediaLocalConsent, now: Long): Boolean =
        !stopping && audioPermissionControl()?.audioPermissionResult(prompt, granted, choice, now) == true
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
