package com.blent.connection

import java.io.DataInputStream
import java.net.Socket
import kotlinx.coroutines.*
import org.junit.Assert.*
import org.junit.Test

private class CameraActorLoopback {
    val protocol = TabletSessionRig(capabilities = 3, timeout = 60000)
    val setup = RuntimeSetup().apply { offer() }
    val events = ArrayList<ConnectionWorkerEvent>()
    var opened = 0
    val native = ArrayList<LocalEndpointOwner>()
    var closed = false
    private val origin = System.nanoTime()
    fun clock(): Long = ((System.nanoTime() - origin) / 1_000_000).also { check(it < 7000) }
    val local = ConnectionLocalResources(protocol.tablet, 4096, ConnectionRunLimits(1000, 1000),
        factory = { now -> opened++; LocalEndpointOwner(LocalSocketEndpoint(), now, ConnectionRunLimits(1000, 1000)).also(native::add) },
        cameraAuthority = protocol.tablet)
    private val transport = object : ConnectionRun {
        override fun display(): LocalDisplayRoute? = null
        override fun endpoints(): ConnectionEndpoints = protocol.tablet
        override fun advance(now: Long) {
            protocol.now = now
            if (!closed) { protocol.tablet.tick(now); protocol.pump() }
        }
        override fun stop(now: Long) {
            if (closed) return
            closed = true; protocol.now = now; protocol.tablet.disconnect(); protocol.tablet.transportRetired()
        }
        override fun failure(): Exception? = null
        override fun retired() = closed
        override fun terminationRequired() = false
    }
    var proofAccepted: Boolean? = null
    private val observedLocal = object : LocalConnectionResources by local, LocalCameraControl by local {
        override fun consumerRetired(lease: ConnectionResourceLease, now: Long): Boolean =
            local.consumerRetired(lease, now).also { proofAccepted = it }
    }
    private val run = ConnectionLocalRun(transport, observedLocal)
    val worker = AndroidConnectionWorker(ConnectionWorkerSpec("usb", io = ConnectionRunLimits(1000, 1000), timerMs = 1),
        { _, _ -> object : ConnectionSetup by setup {
            override fun stop() { setup.stop(); setup.gone = true }
        } }, events::add, ::clock, ConnectionRunFactory { run })
}

class ConnectionCameraRuntimeLoopbackTest {
    private suspend fun until(done: () -> Boolean) { withTimeout(4000) { while (!done()) delay(1) } }
    @Test fun t940RealActorConsentTcpAndRetainedNoHardwareConsumerCleanup() = runBlocking {
        val rig = CameraActorLoopback(); var socket: Socket? = null
        var lease: ConnectionResourceLease? = null
        rig.protocol.send(rig.protocol.invitation(ConnectionManagement.Resource.CAMERA))
        val execution = async { rig.worker.execute() }
        try {
            until { rig.events.any { it is ConnectionWorkerEvent.CameraOffered } }
            val offer = (rig.events.first() as ConnectionWorkerEvent.CameraOffered).offer; lease = offer.lease
            assertEquals(0, rig.opened)
            assertTrue(rig.worker.command(ConnectionWorkerCommand.CameraChoice(offer.lease, CameraConsentChoice(true, false))))
            until { rig.events.any { it is ConnectionWorkerEvent.CameraOffered && it.offer.permissionRequired } }
            assertEquals(0, rig.opened)
            assertTrue(rig.worker.command(ConnectionWorkerCommand.CameraPermission(offer.lease, true, CameraConsentChoice(true, true))))
            until { rig.protocol.messages.any { it == ConnectionManagement.Prepared(offer.lease.key) } }
            rig.protocol.send(ConnectionManagement.Commit(offer.lease.key)); rig.protocol.host.activate(ConnectionWire.CAMERA, 1)
            until { rig.events.any { it is ConnectionWorkerEvent.Camera } }
            val route = (rig.events.last() as ConnectionWorkerEvent.Camera).route
            assertEquals(1, rig.opened); socket = Socket("127.0.0.1", route.port).apply { soTimeout = 1000 }
            val greeting = "BLCAM002".toByteArray() + "0".repeat(64).toByteArray() + byteArrayOf(0, 0)
            socket.getOutputStream().write(greeting)
            until { rig.protocol.payloads.sumOf { it.second.size } == greeting.size }
            assertArrayEquals(greeting, rig.protocol.payloads.flatMap { it.second.toList() }.toByteArray())
            assertTrue(rig.protocol.host.offer(ConnectionWire.CAMERA, "OK".toByteArray())); delay(32)
            val feedback = ByteArray(2); DataInputStream(socket.getInputStream()).readFully(feedback)
            assertArrayEquals("OK".toByteArray(), feedback)
            rig.worker.cancel(); until { rig.events.lastOrNull() == ConnectionWorkerEvent.CameraClosed(offer.lease) }
            assertFalse(execution.isCompleted); socket.close(); socket = null
            // This fixture never started hardware capture. The completed socket
            // operation supplies explicit no-hardware-start proof, not TCP EOF.
            assertTrue(rig.worker.command(ConnectionWorkerCommand.CameraConsumerRetired(offer.lease)))
            execution.await()
            try { rig.worker.retire() } catch (error: Exception) {
                throw AssertionError("T940 local retired=${rig.local.retired()}, quarantine=${rig.local.quarantined()}, native=${rig.native.map { it.retired() to it.quarantined() }}, closed=${rig.closed}, setupGone=${rig.setup.gone}, proof=${rig.proofAccepted}", error)
            }
            assertEquals(true, rig.proofAccepted); assertTrue(rig.local.retired())
            assertFalse(rig.local.quarantined())
        } finally {
            socket?.close(); rig.worker.cancel()
            lease?.let { rig.worker.command(ConnectionWorkerCommand.CameraConsumerRetired(it)) }
            execution.await()
        }
    }
}
