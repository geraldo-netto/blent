package com.blent.connection

import java.io.IOException
import java.util.ArrayDeque
import org.junit.Assert.*
import org.junit.Test

internal class ResourceEndpoints : ConnectionEndpoints by RunnerSession() {
    val events = ArrayDeque<ConnectionSessionEvent>()
    val prepared = ArrayList<ConnectionResourceLease>()
    val retired = ArrayList<ConnectionResourceLease>()
    val cancelled = ArrayList<ConnectionResourceLease>()
    val rejected = ArrayList<ConnectionResourceLease>()
    val active = HashSet<ConnectionResourceLease>()
    override fun nextEvent() = events.pollFirst()
    override fun read(lease: ConnectionResourceLease, channel: Int, maxBytes: Int): ConnectionChannelRead? = null
    override fun offer(lease: ConnectionResourceLease, channel: Int, bytes: ByteArray) = lease in active
    override fun prepared(lease: ConnectionResourceLease, now: Long) { prepared.add(lease) }
    override fun current(lease: ConnectionResourceLease) = lease in active
    override fun cancel(lease: ConnectionResourceLease, reason: ConnectionManagement.Reason) {
        cancelled.add(lease); active.remove(lease)
        state(lease, ConnectionResourceState.CLOSING)
    }
    override fun reject(lease: ConnectionResourceLease, outcome: ConnectionManagement.Outcome) { rejected.add(lease) }
    override fun nativeRetired(lease: ConnectionResourceLease) { retired.add(lease) }
    fun state(lease: ConnectionResourceLease, state: ConnectionResourceState) {
        if (state == ConnectionResourceState.ADMITTED) active.add(lease)
        if (state != ConnectionResourceState.ADMITTED) active.remove(lease)
        events.add(ConnectionSessionEvent.State(lease, state))
    }
}
internal class PromptEndpoint(private val number: Int) : LocalEndpoint {
    val io = RunnerIo()
    var bound = true
    var admitted = false
    var stopped = false
    var gone = false
    var taken = false
    var failure: Exception? = null
    override fun advance(now: Long) {}
    override fun admit(now: Long) { check(!admitted && !stopped); admitted = true }
    override fun port(): Int? = if (bound && !stopped) number else null
    override fun takeIo(): ConnectionIo? = if (taken) null else io.also { taken = true }
    override fun stop(now: Long) { stopped = true; io.stop() }
    override fun failure() = failure
    override fun retired() = stopped && gone
    override fun quarantined() = stopped && !gone
    fun finish() { gone = true; io.gone = true }
}
internal class ResourceRig(factory: ((Long) -> LocalEndpoint)? = null) {
    val endpoint = ResourceEndpoints()
    val lease = LocalPumpRig().lease
    val credential = ConnectionManagement.Credential(ByteArray(32) { 42 })
    val native = ArrayList<PromptEndpoint>()
    val local = ConnectionLocalResources(endpoint, 4096, ConnectionRunLimits(10, 5), factory ?: {
        PromptEndpoint(20000 + native.size).also { native.add(it) }
    })
    fun offer() { endpoint.events.add(ConnectionSessionEvent.Offered(lease, credential, ConnectionManagement.Display)); local.advance(0) }
    fun admit() { endpoint.state(lease, ConnectionResourceState.ADMITTED); local.advance(1) }
}

class ConnectionLocalResourcesTest {
    @Test fun t906OnlyPreparedCurrentAdmissionPublishesDistinctLocalPortsAndStartsPumps() {
        val rig = ResourceRig(); rig.offer()
        assertEquals(listOf(rig.lease), rig.endpoint.prepared); assertNull(rig.local.display())
        assertTrue(rig.native.none { it.taken }); rig.admit()
        val route = checkNotNull(rig.local.display())
        assertEquals(rig.lease, route.lease); assertSame(rig.credential, route.credential)
        assertEquals(20000, route.controlPort); assertEquals(20001, route.displayPort)
        assertTrue(rig.native.all { it.admitted && it.taken })
        assertEquals(4096, rig.native[0].io.read().loan.length)
        assertFalse(rig.local.retired()); assertFalse(rig.local.quarantined())
        rig.local.advance(2); assertEquals(1, rig.endpoint.prepared.size)
    }
    @Test fun t906BothNativeOwnersAndPumpsMustRetireBeforeLeaseRelease() {
        val rig = ResourceRig(); rig.offer(); rig.admit()
        rig.endpoint.state(rig.lease, ConnectionResourceState.CLOSING)
        assertNull(rig.local.display()); rig.local.advance(2)
        assertTrue(rig.native.all { it.stopped }); assertTrue(rig.local.quarantined())
        rig.native[0].finish(); rig.local.advance(3); assertTrue(rig.endpoint.retired.isEmpty())
        rig.native[1].finish(); rig.local.advance(4)
        assertEquals(listOf(rig.lease), rig.endpoint.retired)
        rig.local.advance(5); assertEquals(1, rig.endpoint.retired.size)
        rig.endpoint.state(rig.lease, ConnectionResourceState.RETIRED); rig.local.advance(6)
        rig.local.stop(6); assertTrue(rig.local.retired()); assertFalse(rig.local.quarantined())
    }
    @Test fun t906StaleCallbacksCannotStopOrRetireCurrentDisplay() {
        val rig = ResourceRig(); rig.offer(); rig.admit()
        val stale = rig.lease.copy(owner = 2)
        rig.endpoint.state(stale, ConnectionResourceState.CLOSING)
        rig.endpoint.state(stale, ConnectionResourceState.RETIRED)
        rig.endpoint.events.add(ConnectionSessionEvent.Readable(stale, 1))
        rig.local.advance(2)
        assertNotNull(rig.local.display()); assertTrue(rig.native.none { it.stopped })
        rig.endpoint.state(rig.lease, ConnectionResourceState.RETIRED)
        assertThrows(IllegalStateException::class.java) { rig.local.advance(3) }
    }
    @Test fun t906BindFailureCancelsOnlyItsLeaseAndRetainsSiblingNativeOwnership() {
        val rig = ResourceRig(); rig.offer()
        rig.native[0].failure = IOException("bind failed"); rig.local.advance(1)
        assertEquals(listOf(rig.lease), rig.endpoint.cancelled)
        assertTrue(rig.native.all { it.stopped }); assertTrue(rig.endpoint.retired.isEmpty())
        rig.native.forEach { it.finish() }; rig.local.advance(2)
        assertEquals(listOf(rig.lease), rig.endpoint.retired); assertNull(rig.local.display())
    }
    @Test fun t906PartialFactoryFailureNeverAdmitsReplacementEvenAfterReturnedOwnerRetires() {
        val owned = PromptEndpoint(23456); var calls = 0
        val rig = ResourceRig { if (calls++ == 0) owned else throw IOException("partial native work") }
        rig.offer(); assertTrue(owned.stopped); assertTrue(rig.local.quarantined())
        owned.finish(); rig.local.advance(1); rig.local.stop(2)
        assertFalse(rig.local.retired()); assertTrue(rig.endpoint.retired.isEmpty())
        rig.endpoint.events.add(ConnectionSessionEvent.Offered(rig.lease.copy(owner = 2), rig.credential, ConnectionManagement.Display))
        assertThrows(IllegalStateException::class.java) { rig.local.advance(3) }
    }
    @Test fun t906UnavailableResourcesCannotAcquireNativeOrDisruptDisplay() {
        val rig = ResourceRig(); rig.offer(); rig.admit()
        val camera = rig.lease.copy(key = rig.lease.key.copy(resource = ConnectionManagement.Resource.CAMERA))
        rig.endpoint.events.add(ConnectionSessionEvent.Offered(camera, rig.credential,
            ConnectionManagement.Camera(640, 480, 30, 100000, 10000, 100, 0, false, false)))
        rig.endpoint.state(camera, ConnectionResourceState.CLOSING)
        rig.endpoint.state(camera, ConnectionResourceState.RETIRED)
        rig.local.advance(2)
        assertEquals(listOf(camera), rig.endpoint.rejected); assertEquals(listOf(camera), rig.endpoint.retired)
        assertEquals(2, rig.native.size); assertNotNull(rig.local.display())
    }
    @Test fun t906StopDrainsLateOffersWithoutNativeCreationAndBoundsEventWork() {
        val rig = ResourceRig(); rig.local.stop(0)
        repeat(40) { rig.endpoint.events.add(ConnectionSessionEvent.Readable(rig.lease, 1)) }
        rig.local.advance(0); assertEquals(8, rig.endpoint.events.size)
        rig.offer(); assertTrue(rig.native.isEmpty()); assertEquals(listOf(rig.lease), rig.endpoint.rejected)
        assertTrue(rig.local.retired())
    }
    @Test fun t906BothListenersMustBindBeforePreparedAndDuplicatePortsReject() {
        val rig = ResourceRig { PromptEndpoint(12345).apply { bound = false } }
        rig.offer(); assertTrue(rig.endpoint.prepared.isEmpty())
        val duplicate = ResourceRig { PromptEndpoint(12345) }; duplicate.offer()
        assertTrue(duplicate.endpoint.prepared.isEmpty()); assertEquals(listOf(duplicate.lease), duplicate.endpoint.cancelled)
    }
    @Test fun t906PumpFailureStopsDisplaySiblingAndHidesRouteImmediately() {
        val rig = ResourceRig(); rig.offer(); rig.admit()
        val io = rig.native[0].io; io.complete(io.read(), -1); rig.local.advance(2)
        assertTrue(rig.native.all { it.stopped }); assertNull(rig.local.display())
        assertEquals(listOf(rig.lease), rig.endpoint.cancelled)
        rig.native.forEach { it.finish() }; rig.local.advance(3)
        assertEquals(listOf(rig.lease), rig.endpoint.retired)
    }
    @Test fun t906MalformedRoutesCapacitiesAndClocksReject() {
        val rig = ResourceRig()
        for (port in listOf(Int.MIN_VALUE, -1, 0, 65536, Int.MAX_VALUE)) {
            assertThrows(IllegalArgumentException::class.java) { LocalDisplayRoute(rig.lease, rig.credential, port, 5) }
            assertThrows(IllegalArgumentException::class.java) { LocalDisplayRoute(rig.lease, rig.credential, 5, port) }
        }
        assertThrows(IllegalArgumentException::class.java) { LocalDisplayRoute(rig.lease, rig.credential, 5, 5) }
        for (now in listOf(Long.MIN_VALUE, -1, Long.MAX_VALUE)) {
            assertThrows(IllegalArgumentException::class.java) { rig.local.advance(now) }
        }
        for (capacity in listOf(Int.MIN_VALUE, -1, 0, Int.MAX_VALUE)) {
            assertThrows(IllegalArgumentException::class.java) { ConnectionLocalResources(rig.endpoint, capacity) }
        }
        rig.local.advance(3); assertThrows(IllegalArgumentException::class.java) { rig.local.stop(2) }
    }
    @Test fun t906ComposedRunKeepsResourceQuarantineLocalUntilConnectionStops() {
        val rig = ResourceRig(); val native = RunnerIo(); val session = RunnerSession()
        val transport = ConnectionSessionRunner(session, native)
        val run = ConnectionLocalRun(transport, rig.local)
        assertSame(session, run.endpoints()); rig.offer(); rig.admit()
        rig.native[0].failure = IOException("local failed"); run.advance(2)
        assertNull(run.failure()); assertFalse(run.terminationRequired()); assertFalse(run.retired())
        run.stop(3); run.advance(4); assertTrue(run.terminationRequired()); assertTrue(native.stopping)
        rig.native.forEach { it.finish() }; native.gone = true; run.advance(5)
        assertTrue(run.retired()); assertFalse(run.terminationRequired())
    }
    @Test fun t906TransportFailureRevokesLocalRouteAndRetainsOwnership() {
        val rig = ResourceRig(); val native = RunnerIo(); val transport = ConnectionSessionRunner(RunnerSession(), native)
        val run = ConnectionLocalRun(transport, rig.local)
        rig.offer(); rig.admit(); run.advance(2)
        native.complete(native.read(), -1); run.advance(3)
        assertNotNull(run.failure()); assertNull(rig.local.display()); assertTrue(rig.native.all { it.stopped })
        native.gone = true; run.advance(4); assertFalse(run.retired())
    }
    @Test fun t906GroupRetainsOriginalFailureAndFailedStopOwnership() {
        val rig = ResourceRig(); val source = PromptEndpoint(12345)
        val failure = IOException("native stop failed")
        val native = object : LocalEndpoint by source { override fun stop(now: Long) { throw failure } }
        val offer = ConnectionSessionEvent.Offered(rig.lease, rig.credential, ConnectionManagement.Display)
        val group = LocalResourceGroup(offer, rig.endpoint, 4096, ConnectionRunLimits())
        assertNull(group.failure()); group.open(0) { native }
        group.advance(0) // Duplicate listener ports rejected before preparation.
        assertSame(failure, group.failure()); assertTrue(group.quarantined())
        source.finish(); group.advance(1); assertFalse(group.retired()); assertNull(group.route())
        group.admit(1); assertNull(group.route()) // Late admission after stop.
    }
    @Test fun t906InvalidLeaseCountersRejectRouteWithoutLocalAuthority() {
        val rig = ResourceRig(); val random = java.util.Random(906)
        for (value in listOf(Long.MIN_VALUE, -1, 0) + List(64) { random.nextLong() or Long.MIN_VALUE }) {
            val leases = listOf(rig.lease.copy(owner = value), rig.lease.copy(connection = rig.lease.connection.copy(epoch = value)),
                rig.lease.copy(key = rig.lease.key.copy(request = value)), rig.lease.copy(key = rig.lease.key.copy(generation = value)))
            for (lease in leases) {
                assertThrows(IllegalArgumentException::class.java) { LocalDisplayRoute(lease, rig.credential, 1, 65535) }
            }
        }
        LocalDisplayRoute(rig.lease.copy(owner = Long.MAX_VALUE), rig.credential, 65535, 1)
    }

}
