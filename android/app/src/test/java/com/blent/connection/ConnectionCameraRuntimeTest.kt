package com.blent.connection

import kotlinx.coroutines.*
import org.junit.Assert.*
import org.junit.Test

internal class CameraRuntimeRun : ConnectionRun, LocalCameraControl {
    val endpoints = ResourceEndpoints()
    var offer: LocalCameraOffer? = LocalCameraOffer(cameraLease(), cameraProfile(), false)
    var route: LocalCameraRoute? = null
    var prompt: CameraPermissionRequest? = null
    var stopped = false
    var advanced = false
    var gone = false
    val choices = ArrayList<CameraConsentChoice>()
    var replies = 0
    var proofs = 0
    override fun display(): LocalDisplayRoute? = null
    override fun endpoints(): ConnectionEndpoints = endpoints
    override fun advance(now: Long) { advanced = true }
    override fun stop(now: Long) { stopped = true }
    override fun failure(): Exception? = null
    override fun retired() = gone
    override fun terminationRequired() = false
    override fun offered() = offer
    override fun camera() = route
    override fun pending() = prompt
    override fun choose(lease: ConnectionResourceLease, choice: CameraConsentChoice, now: Long): Boolean {
        if (lease != cameraLease()) return false
        choices.add(choice); return true
    }
    override fun permissionResult(prompt: CameraPermissionRequest, granted: Boolean, choice: CameraConsentChoice, now: Long): Boolean {
        if (prompt !== this.prompt) return false
        replies++; this.prompt = null; return true
    }
    override fun consumerRetired(lease: ConnectionResourceLease, now: Long): Boolean {
        if (!stopped || lease != cameraLease() || gone) return false
        proofs++; gone = true; return true
    }
}

class ConnectionCameraRuntimeTest {
    private suspend fun until(condition: () -> Boolean) { withTimeout(2000) { while (!condition()) delay(1) } }
    @Test fun t940ActorKeepsTerminalCaptureCleanupAfterCancellation() = runBlocking {
        val setup = RuntimeSetup().apply { offer() }; val run = CameraRuntimeRun()
        val worker = AndroidConnectionWorker(ConnectionWorkerSpec("usb", io = ConnectionRunLimits(100, 500), timerMs = 1),
            { _, _ -> object : ConnectionSetup by setup {
                override fun stop() { setup.stop(); setup.gone = true }
            } }, {}, { 0 }, ConnectionRunFactory { run })
        val execution = async { worker.execute() }
        try {
            until { run.advanced }; worker.cancel(); until { run.stopped }
            assertFalse(worker.command(ConnectionWorkerCommand.CameraChoice(cameraLease(), CameraConsentChoice(true, true))))
            assertTrue("Exact retained cleanup must reach the stopping actor",
                worker.command(ConnectionWorkerCommand.CameraConsumerRetired(cameraLease())))
            execution.await(); worker.retire(); assertEquals(1, run.proofs)
            assertFalse(worker.command(ConnectionWorkerCommand.CameraConsumerRetired(cameraLease())))
        } finally { run.gone = true; worker.cancel(); execution.await() }
    }
    private fun owner(setup: RuntimeSetup, run: ConnectionRun): ConnectionRuntimeOwner {
        setup.offer()
        return ConnectionRuntimeOwner(setup, ConnectionRunFactory { run }, { 0 }).also { it.advance() }
    }
    @Test fun t940RuntimeGuardsPendingAdmissionAndOpaquePermissionReplies() {
        val setup = RuntimeSetup(); val run = CameraRuntimeRun(); val runtime = owner(setup, run)
        assertEquals(run.offer, runtime.offered()); assertNull(runtime.camera())
        val choice = CameraConsentChoice(true, false)
        assertTrue(runtime.command(ConnectionWorkerCommand.CameraChoice(cameraLease(), choice)))
        assertEquals(listOf(choice), run.choices)
        val response = ConnectionWorkerCommand.CameraPermission(cameraLease(), true, CameraConsentChoice(true, true))
        assertFalse(runtime.command(response)); run.prompt = CameraPermissionRequest(cameraLease())
        assertFalse(runtime.command(response.copy(lease = cameraLease(2))))
        assertTrue(runtime.command(response)); assertEquals(1, run.replies)
        assertFalse(runtime.command(response)); assertEquals(1, run.replies)
        run.offer = null; run.route = cameraRoute()
        assertNull(runtime.offered()); assertNull(runtime.camera())
        run.endpoints.active.add(cameraLease()); assertEquals(run.route, runtime.camera())
        val foreign = cameraLease().copy(connection = setup.ticket.copy(epoch = 2))
        run.route = cameraRoute().copy(lease = foreign); assertNull(runtime.camera())
        run.offer = LocalCameraOffer(foreign, cameraProfile(), false); assertNull(runtime.offered())
        assertFalse(runtime.command(ConnectionWorkerCommand.CameraChoice(foreign, choice)))
    }
    @Test fun t940OriginalAuthorityLossStopsNewControlsButRetainsExactCleanup() {
        val setup = RuntimeSetup(); val run = CameraRuntimeRun(); val runtime = owner(setup, run)
        setup.live = false; assertNull(runtime.offered()); assertTrue(run.stopped)
        assertNull(runtime.camera())
        assertFalse(runtime.command(ConnectionWorkerCommand.CameraChoice(cameraLease(), CameraConsentChoice(true, true))))
        assertFalse(runtime.command(ConnectionWorkerCommand.CameraPermission(cameraLease(), true, CameraConsentChoice(true, true))))
        assertFalse(runtime.command(ConnectionWorkerCommand.CameraConsumerRetired(cameraLease(2))))
        assertTrue(runtime.command(ConnectionWorkerCommand.CameraConsumerRetired(cameraLease())))
        assertFalse(runtime.command(ConnectionWorkerCommand.CameraConsumerRetired(cameraLease())))
        assertEquals(1, run.proofs)
        val pending = ConnectionRuntimeOwner(RuntimeSetup(), ConnectionRunFactory { error("No admission") }, { 0 })
        assertFalse(pending.command(ConnectionWorkerCommand.CameraChoice(cameraLease(), CameraConsentChoice(true, true))))
        assertNull(pending.offered()); assertNull(pending.camera())
    }
    @Test fun t940BoundedForeignLeaseControlsNeverReachOriginalConsent() {
        val setup = RuntimeSetup(); val run = CameraRuntimeRun(); val runtime = owner(setup, run)
        val random = java.util.Random(940)
        run.prompt = CameraPermissionRequest(cameraLease())
        val choices = CameraConsentChoice(true, true)
        val counters = listOf(2L, Long.MAX_VALUE) + List(128) { (random.nextLong() and Long.MAX_VALUE).coerceAtLeast(2) }
        for (counter in counters) {
            val leases = listOf(cameraLease(counter), cameraLease().copy(connection = setup.ticket.copy(epoch = counter)))
            for (lease in leases) {
                assertFalse(runtime.command(ConnectionWorkerCommand.CameraChoice(lease, choices)))
                assertFalse(runtime.command(ConnectionWorkerCommand.CameraPermission(lease, true, choices)))
                assertFalse(runtime.command(ConnectionWorkerCommand.CameraConsumerRetired(lease)))
            }
        }
        assertTrue(run.choices.isEmpty()); assertEquals(0, run.replies); assertEquals(0, run.proofs)
        assertNotNull(run.prompt)
    }
    @Test fun t940LocalCompositionForwardsFocusedControlsAndRetainsCleanup() {
        val camera = CameraRuntimeRun()
        val resources = object : LocalConnectionResources, LocalCameraControl by camera {
            override fun advance(now: Long) { camera.advance(now) }
            override fun display(): LocalDisplayRoute? = null
            override fun stop(now: Long) { camera.stop(now) }
            override fun retired() = camera.retired()
            override fun quarantined() = false
        }
        val run = ConnectionLocalRun(RuntimeRun(), resources)
        assertEquals(camera.offer, run.offered()); assertNull(run.camera())
        val choice = CameraConsentChoice(true, true)
        assertTrue(run.choose(cameraLease(), choice, 0))
        val prompt = CameraPermissionRequest(cameraLease()); camera.prompt = prompt
        assertSame(prompt, run.pending()); assertTrue(run.permissionResult(prompt, true, choice, 0))
        camera.route = cameraRoute(); assertEquals(camera.route, run.camera())
        run.stop(1); assertNull(run.offered()); assertNull(run.pending()); assertNull(run.camera())
        assertFalse(run.choose(cameraLease(), choice, 1)); assertFalse(run.permissionResult(prompt, true, choice, 1))
        assertTrue(run.consumerRetired(cameraLease(), 1))
    }
    @Test fun t940DisplayOnlyCompositionHasNoImplicitCameraControl() {
        val choice = CameraConsentChoice(true, true)
        val prompt = CameraPermissionRequest(cameraLease())
        val displayOnly = ConnectionLocalRun(RuntimeRun(), object : LocalConnectionResources {
            override fun advance(now: Long) {}
            override fun display(): LocalDisplayRoute? = null
            override fun stop(now: Long) {}
            override fun retired() = true
            override fun quarantined() = false
        })
        assertNull(displayOnly.offered()); assertNull(displayOnly.camera()); assertNull(displayOnly.pending())
        assertFalse(displayOnly.choose(cameraLease(), choice, 0))
        assertFalse(displayOnly.permissionResult(prompt, true, choice, 0)); assertFalse(displayOnly.consumerRetired(cameraLease(), 0))
    }
    @Test fun t940ActorPublishesChangesAndRevokesCameraWhenAuthorityDisappears() = runBlocking {
        val setup = RuntimeSetup().apply { offer() }; val run = CameraRuntimeRun()
        val events = ArrayList<ConnectionWorkerEvent>()
        val worker = AndroidConnectionWorker(ConnectionWorkerSpec("usb", timerMs = 1),
            { _, _ -> object : ConnectionSetup by setup {
                override fun stop() { setup.stop(); setup.gone = true; run.gone = true }
            } }, events::add, { 0 }, ConnectionRunFactory { run })
        val execution = async { worker.execute() }
        try {
            until { events.isNotEmpty() }; assertEquals(ConnectionWorkerEvent.CameraOffered(run.offer!!), events.single())
            delay(5); assertEquals(1, events.size)
            run.offer = run.offer!!.copy(permissionRequired = true)
            until { events.size == 2 }; assertEquals(ConnectionWorkerEvent.CameraOffered(run.offer!!), events[1])
            run.offer = null; run.route = cameraRoute(); run.endpoints.active.add(cameraLease())
            until { events.size == 3 }; assertEquals(ConnectionWorkerEvent.Camera(run.route!!), events[2])
            assertTrue(worker.command(ConnectionWorkerCommand.CameraChoice(cameraLease(), CameraConsentChoice(true, true))))
            until { run.choices.isNotEmpty() }; setup.live = false
            execution.await(); worker.retire(); assertEquals(ConnectionWorkerEvent.CameraClosed(cameraLease()), events.last())
            assertEquals(4, events.size)
        } finally { worker.cancel(); execution.await() }
    }
}
