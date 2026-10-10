package com.blent.connection

import java.io.DataInputStream
import java.net.Socket
import org.junit.Assert.*
import org.junit.Test

private class CameraOwnerLoopback {
    val protocol = TabletSessionRig(capabilities = 3, timeout = 60000)
    var opened = 0
    val local = ConnectionLocalResources(protocol.tablet, 4096, factory = { now ->
        opened++; LocalEndpointOwner(LocalSocketEndpoint(), now)
    }, cameraAuthority = protocol.tablet)
    private val origin = System.nanoTime()
    private var closed = false
    fun step() {
        protocol.now = (System.nanoTime() - origin) / 1_000_000
        check(protocol.now < 7000) { "T937 camera control loop stalled" }
        local.advance(protocol.now)
        if (!closed) protocol.pump()
        Thread.sleep(1)
    }
    fun await(done: () -> Boolean) { while (!done()) step() }
    fun close() {
        closed = true
        local.stop(protocol.now); protocol.tablet.disconnect(); protocol.tablet.transportRetired()
    }
}

class LocalCameraOwnerLoopbackTest {
    @Test fun t937RealConsentProtocolAndTcpJoinEndpointWithIndependentConsumerProof() {
        val rig = CameraOwnerLoopback(); var socket: Socket? = null
        var lease: ConnectionResourceLease? = null
        try {
            rig.protocol.send(rig.protocol.invitation(ConnectionManagement.Resource.CAMERA)); rig.step()
            val offered = checkNotNull(rig.local.offered()); lease = offered.lease
            assertEquals(0, rig.opened)
            rig.local.choose(offered.lease, CameraConsentChoice(true, false), rig.protocol.now); rig.step()
            assertTrue(rig.protocol.messages.any { it == ConnectionManagement.ResultMessage(offered.lease.key, ConnectionManagement.Outcome.PERMISSION_REQUIRED) })
            assertEquals(0, rig.opened)
            rig.local.permissionResult(checkNotNull(rig.local.pending()), true, CameraConsentChoice(true, true), rig.protocol.now)
            rig.await { rig.protocol.messages.any { it == ConnectionManagement.Prepared(offered.lease.key) } }
            assertEquals(1, rig.opened)
            rig.protocol.send(ConnectionManagement.Commit(offered.lease.key)); rig.protocol.host.activate(ConnectionWire.CAMERA, 1)
            rig.await { rig.local.camera() != null }
            val route = checkNotNull(rig.local.camera())
            socket = Socket("127.0.0.1", route.port).apply { soTimeout = 1000 }
            val greeting = "BLCAM002".toByteArray() + "0".repeat(64).toByteArray() + byteArrayOf(0, 0)
            socket.getOutputStream().write(greeting)
            rig.await { rig.protocol.payloads.sumOf { it.second.size } == greeting.size }
            assertArrayEquals(greeting, rig.protocol.payloads.flatMap { it.second.toList() }.toByteArray())
            assertTrue(rig.protocol.host.offer(ConnectionWire.CAMERA, "OK".toByteArray()))
            repeat(16) { rig.step() }
            val feedback = ByteArray(2); DataInputStream(socket.getInputStream()).readFully(feedback)
            assertArrayEquals("OK".toByteArray(), feedback)
        } finally {
            socket?.close(); rig.close()
            // The fixture's external consumer is this completed socket operation;
            // no camera hardware worker was started. Production T932 supplies its
            // actual isolated worker completion instead of this no-start proof.
            lease?.let { assertTrue(rig.local.consumerRetired(it, rig.protocol.now)) }
            rig.await { rig.local.retired() }
        }
        assertFalse(rig.local.quarantined()); assertNull(rig.local.camera())
    }
}
