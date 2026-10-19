package com.blent.connection

import android.content.*
import android.os.*
import java.util.concurrent.CopyOnWriteArrayList
import java.util.concurrent.atomic.AtomicInteger
import kotlinx.coroutines.*
import org.robolectric.Robolectric
import org.robolectric.RuntimeEnvironment

/** Real private service/Binder, actor, mux and local TCP. Pairing/USB permission
 * are explicit synthetic authority; process death is simulated, never hardware
 * acceptance. Every protocol mutation shares the actor's synchronization gate. */
internal class ConnectionJoinedCameraFixture(private val capabilities: Int = 3, private val report: (ConnectionWorkerEvent) -> Unit = {}) {
    val gate = Any()
    val protocol = TabletSessionRig(capabilities = capabilities, timeout = 60000)
    val presentation = ConnectionPresentation()
    val token = presentation.begin()
    val opened = AtomicInteger()
    val controls = CopyOnWriteArrayList<ConnectionWorkerCommand>()
    val events = CopyOnWriteArrayList<ConnectionWorkerEvent>()
    val cancellations = CopyOnWriteArrayList<Pair<ConnectionResourceLease, ConnectionManagement.Reason>>()
    val nativeEndpoints = CopyOnWriteArrayList<LocalEndpoint>()
    private val recordedEndpoints = object : ConnectionEndpoints by protocol.tablet {
        override fun cancel(lease: ConnectionResourceLease, reason: ConnectionManagement.Reason) {
            cancellations.add(lease to reason); protocol.tablet.cancel(lease, reason)
        }
    }
    val controller = Robolectric.buildService(ConnectionProbeService::class.java).create()
    val service = controller.get()
    private val origin = System.nanoTime()
    private var closed = false
    private var delivered = false
    private val local = ConnectionLocalResources(recordedEndpoints, 4096, ConnectionRunLimits(5000, 1000),
        factory = { now -> opened.incrementAndGet(); LocalEndpointOwner(LocalSocketEndpoint(), now, ConnectionRunLimits(5000, 1000)).also(nativeEndpoints::add) },
        cameraAuthority = protocol.tablet, audioAuthority = protocol.tablet,
        microphoneAuthority = if (capabilities and 4 != 0) protocol.tablet else null)
    private val transport = object : ConnectionRun {
        override fun display(): LocalDisplayRoute? = null
        override fun endpoints(): ConnectionEndpoints = protocol.tablet
        override fun advance(now: Long) {
            protocol.now = now
            if (!closed) { protocol.tablet.tick(now); protocol.pump() }
        }
        override fun stop(now: Long) {
            if (closed) return
            closed = true; protocol.tablet.disconnect(); protocol.tablet.transportRetired()
        }
        override fun failure(): Exception? = null
        override fun retired() = closed
        override fun terminationRequired() = false
    }
    private val joined = ConnectionLocalRun(transport, local)
    private val serialized = object : ConnectionRun, LocalCameraControl, LocalAudioControl, LocalAudioPermissionControl {
        override fun display() = synchronized(gate) { joined.display() }
        override fun endpoints(): ConnectionEndpoints = object : ConnectionEndpoints by protocol.tablet {
            override fun current(lease: ConnectionResourceLease) = synchronized(gate) { protocol.tablet.current(lease) }
        }
        override fun advance(now: Long) = synchronized(gate) { joined.advance(now) }
        override fun stop(now: Long) = synchronized(gate) { joined.stop(now) }
        override fun failure() = synchronized(gate) { joined.failure() }
        override fun retired() = synchronized(gate) { joined.retired() }
        override fun terminationRequired() = synchronized(gate) { joined.terminationRequired() }
        override fun offered() = synchronized(gate) { joined.offered() }
        override fun camera() = synchronized(gate) { joined.camera() }
        override fun pending() = synchronized(gate) { joined.pending() }
        override fun choose(lease: ConnectionResourceLease, choice: CameraConsentChoice, now: Long) =
            synchronized(gate) { joined.choose(lease, choice, now) }
        override fun permissionResult(prompt: CameraPermissionRequest, granted: Boolean, choice: CameraConsentChoice, now: Long) =
            synchronized(gate) { joined.permissionResult(prompt, granted, choice, now) }
        override fun consumerRetired(lease: ConnectionResourceLease, now: Long) = synchronized(gate) { joined.consumerRetired(lease, now) }
        override fun audioOffered(resource: ConnectionManagement.Resource) = synchronized(gate) { joined.audioOffered(resource) }
        override fun audio(resource: ConnectionManagement.Resource) = synchronized(gate) { joined.audio(resource) }
        override fun chooseAudio(lease: ConnectionResourceLease, choice: com.blent.MediaLocalConsent, now: Long) =
            synchronized(gate) { joined.chooseAudio(lease, choice, now) }
        override fun audioConsumerRetired(lease: ConnectionResourceLease, now: Long) = synchronized(gate) { joined.audioConsumerRetired(lease, now) }
        override fun pendingAudio() = synchronized(gate) { joined.pendingAudio() }
        override fun audioPermissionResult(prompt: AudioPermissionRequest, granted: Boolean, choice: com.blent.MediaLocalConsent, now: Long) =
            synchronized(gate) { joined.audioPermissionResult(prompt, granted, choice, now) }
    }
    private val admission = object : AccessoryAdmission {
        private val selection = AccessorySelection(7, "ab".repeat(32))
        @Volatile private var stopped = false
        override fun advance() {}
        override fun takeReady(): AccessorySelection? = if (stopped || delivered) null else selection.also { delivered = true }
        override fun current(selection: AccessorySelection) = !stopped && selection == this.selection
        override fun stop() { stopped = true }
        override fun retired() = stopped
        override fun failure(): Exception? = null
        override fun terminationRequired() = false
    }
    private val context = object : ContextWrapper(RuntimeEnvironment.getApplication()) {
        override fun getApplicationContext(): Context = this
        override fun bindService(intent: Intent, connection: ServiceConnection, flags: Int): Boolean {
            val original = service.onBind(intent)
            val endpoint = object : Binder() {
                override fun isBinderAlive() = service.exits.get() == 0
                override fun onTransact(code: Int, data: Parcel, reply: Parcel?, flags: Int) = original.transact(code, data, reply, flags)
            }
            connection.onServiceConnected(intent.component, endpoint)
            return true
        }
        override fun unbindService(connection: ServiceConnection) {}
    }
    val worker = ConnectionAdmissionRun(ConnectionWorkerSpec("template", session = ConnectionSessionLimits(4096),
        io = ConnectionRunLimits(5000, 1000), timerMs = 1), { admission },
        { spec, report -> AndroidConnectionWorkerRun(context, spec, report, bindMillis = 1000, startMillis = 1000, retireMillis = 4000) },
        { events.add(it); presentation.event(token, it); report(it) }, clock = ::clock)
    init {
        service.factory = { spec, report ->
            val setup = RuntimeSetup().apply {
                offer(); events.add(ConnectionWorkerEvent.Peer(ticket, ConnectionPeer.parse("cd".repeat(32)), false))
                events.add(ConnectionWorkerEvent.State(ConnectionWorkerPhase.AUTHENTICATED))
            }
            val actor = AndroidConnectionWorker(spec, { _, _ -> object : ConnectionSetup by setup {
                override fun stop() { setup.stop(); setup.gone = true }
            } }, report, ::clock, ConnectionRunFactory { serialized })
            object : ConnectionWorker by actor {
                override fun command(value: ConnectionWorkerCommand): Boolean { controls.add(value); return actor.command(value) }
            }
        }
    }
    private fun clock() = ((System.nanoTime() - origin) / 1_000_000).also { check(it < 10000) }
    fun send(message: ConnectionManagement.Message) = synchronized(gate) {
        check(protocol.host.offer(ConnectionWire.MANAGEMENT, ConnectionManagement.encode(message)))
    }
    fun commit(lease: ConnectionResourceLease) = synchronized(gate) {
        send(ConnectionManagement.Commit(lease.key))
        for (channel in ConnectionChannels.members(lease.key.resource)) protocol.host.activate(channel, lease.key.generation)
    }
    fun messages() = synchronized(gate) { protocol.messages.toList() }
    fun payload(channel: Int = ConnectionWire.CAMERA) = synchronized(gate) { protocol.payloads.filter { it.first == channel }.flatMap { it.second.toList() }.toByteArray() }
    fun feedback(bytes: ByteArray, channel: Int = ConnectionWire.CAMERA) = synchronized(gate) { protocol.host.offer(channel, bytes) }
    fun retired() = synchronized(gate) { local.retired() }
    fun cleanup() { service.onDestroy() }
}
