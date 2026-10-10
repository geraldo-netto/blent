package com.blent.connection

import java.io.DataInputStream
import java.net.ServerSocket
import java.net.Socket
import org.junit.Assert.*
import org.junit.Test

/** Actual JVM TCP/pumps and production camera management. Camera capture and
 * Android OS permission/worker acceptance stay under T932/T858. */
private class CameraLoopbackRig : CameraConsentAuthority {
    val protocol = TabletSessionRig(capabilities = 3, timeout = 60000)
    val lease = protocol.offered(ConnectionManagement.Resource.CAMERA)
    val offer = protocol.invitation(ConnectionManagement.Resource.CAMERA).let {
        ConnectionSessionEvent.Offered(lease, it.credential, it.profile)
    }
    private val origin = System.nanoTime()
    var owned = true
    var opened = 0
    val choice = CameraConsentChoice(true, true)
    val consent = ConnectionCameraConsent(offer, this)
    val group = LocalResourceGroup(offer, protocol.tablet, 4096, ConnectionRunLimits(), CameraLocalPermit(consent) { choice })
    init {
        check(consent.update(choice) is CameraConsentAction.Prepare)
        group.open(0) { now -> opened++; LocalEndpointOwner(LocalSocketEndpoint(), now) }
    }
    override fun owns(lease: ConnectionResourceLease) = owned && this.lease == lease
    override fun admitted(lease: ConnectionResourceLease) = protocol.tablet.current(lease)
    fun step() {
        protocol.now = (System.nanoTime() - origin) / 1_000_000
        check(protocol.now < 7000) { "T935 actual camera channel stalled" }
        if (owned) protocol.pump()
        for (event in protocol.events()) state(event)
        group.advance(protocol.now)
        if (owned) protocol.pump()
        Thread.sleep(1)
    }
    private fun state(event: ConnectionSessionEvent) {
        if (event !is ConnectionSessionEvent.State) return
        check(event.lease == lease)
        when (event.state) {
            ConnectionResourceState.ADMITTED -> group.admit(protocol.now)
            ConnectionResourceState.CLOSING -> group.stop(protocol.now)
            ConnectionResourceState.RETIRED -> check(group.retired())
        }
    }
    fun await(done: () -> Boolean) { while (!done()) step() }
    fun stop() {
        owned = false; consent.connectionLost(); group.stop(protocol.now)
        protocol.tablet.disconnect(); protocol.tablet.transportRetired()
        await { group.retired() }
        assertFalse(group.quarantined())
    }
}

class LocalCameraLoopbackTest {
    @Test fun t935RealCameraGreetingAndFeedbackUseOneLeasedLoopbackChannel() {
        val rig = CameraLoopbackRig()
        var socket: Socket? = null
        var port: Int
        try {
            rig.await { rig.protocol.messages.any { it == ConnectionManagement.Prepared(rig.lease.key) } }
            assertEquals(1, rig.opened); assertNull(rig.group.cameraRoute())
            rig.protocol.send(ConnectionManagement.Commit(rig.lease.key))
            rig.protocol.host.activate(ConnectionWire.CAMERA, rig.lease.key.generation)
            rig.await { rig.group.cameraRoute() != null }
            val route = checkNotNull(rig.group.cameraRoute()); port = route.port
            assertEquals(rig.offer.profile, route.profile); assertNull(rig.group.route())
            socket = Socket("127.0.0.1", port).apply { soTimeout = 1000 }
            val greeting = "BLCAM002".toByteArray() + "0".repeat(64).toByteArray() + byteArrayOf(0, 0)
            socket.getOutputStream().write(greeting)
            rig.await { rig.protocol.payloads.sumOf { it.second.size } == greeting.size }
            assertTrue(rig.protocol.payloads.all { it.first == ConnectionWire.CAMERA })
            assertArrayEquals(greeting, rig.protocol.payloads.flatMap { it.second.toList() }.toByteArray())
            assertTrue(rig.protocol.host.offer(ConnectionWire.CAMERA, "OK".toByteArray()))
            repeat(16) { rig.step() }
            val feedback = ByteArray(2); DataInputStream(socket.getInputStream()).readFully(feedback)
            assertArrayEquals("OK".toByteArray(), feedback)
        } finally { socket?.close(); rig.stop() }
        assertNull(rig.group.cameraRoute())
        ServerSocket(port, 1, java.net.InetAddress.getByName("127.0.0.1")).close()
    }
}
