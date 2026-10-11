package com.blent.connection

import java.io.DataInputStream
import java.net.Socket
import kotlinx.coroutines.*
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [27, 34])
class ConnectionJoinedCameraTest {
    private suspend fun until(done: () -> Boolean) { withTimeout(4000) { while (!done()) delay(1) } }
    private fun command(rig: ConnectionJoinedCameraFixture, value: ConnectionWorkerCommand?) {
        assertTrue(rig.worker.command(checkNotNull(value)))
    }
    @Test fun t936PresentationBinderActorAndRealCameraTcpRetainCleanupAfterCancellation() = runBlocking {
        val rig = ConnectionJoinedCameraFixture(); var socket: Socket? = null
        var lease: ConnectionResourceLease? = null
        rig.send(rig.protocol.invitation(ConnectionManagement.Resource.CAMERA))
        val execution = async { rig.worker.execute() }
        try {
            until { rig.presentation.state.value.cameraOffer != null }
            val offer = checkNotNull(rig.presentation.state.value.cameraOffer); lease = offer.lease
            assertEquals(0, rig.opened.get())
            command(rig, rig.presentation.cameraChoice(rig.token, offer, CameraConsentChoice(true, false)))
            until { rig.presentation.state.value.cameraOffer?.offer?.permissionRequired == true }
            val pending = checkNotNull(rig.presentation.state.value.cameraOffer)
            assertEquals(0, rig.opened.get())
            command(rig, rig.presentation.cameraPermission(rig.token, pending, true, CameraConsentChoice(true, true)))
            until { rig.messages().contains(ConnectionManagement.Prepared(offer.lease.key)) }
            rig.commit(offer.lease); until { rig.presentation.state.value.camera != null }
            val route = checkNotNull(rig.presentation.state.value.camera).route
            assertEquals(1, rig.opened.get())
            socket = Socket("127.0.0.1", route.port).apply { soTimeout = 1000 }
            val greeting = "BLCAM002".toByteArray() + "0".repeat(64).toByteArray() + byteArrayOf(0, 0)
            socket.getOutputStream().write(greeting); until { rig.payload().size == greeting.size }
            assertArrayEquals(greeting, rig.payload()); assertTrue(rig.feedback("OK".toByteArray()))
            delay(64); val feedback = ByteArray(2); DataInputStream(socket.getInputStream()).readFully(feedback)
            assertArrayEquals("OK".toByteArray(), feedback)
            rig.worker.cancel(); assertNull(rig.presentation.state.value.camera)
            socket.close(); socket = null
            assertTrue(rig.worker.command(ConnectionWorkerCommand.CameraConsumerRetired(offer.lease)))
            withTimeout(5000) { execution.await(); rig.worker.retire() }
            assertTrue(rig.retired()); assertTrue(rig.controls.any { it is ConnectionWorkerCommand.CameraConsumerRetired })
            assertNull(rig.presentation.cameraPermission(rig.token, pending, true, CameraConsentChoice(true, true)))
        } finally {
            socket?.close(); rig.worker.cancel()
            lease?.let { rig.worker.command(ConnectionWorkerCommand.CameraConsumerRetired(it)) }
            withTimeout(5000) { execution.await(); rig.worker.retire() }; rig.cleanup()
        }
    }
    @Test fun t936DeniedCameraPermissionClosesOnlyCameraAndOpensNoCameraEndpoint() = runBlocking {
        val rig = ConnectionJoinedCameraFixture()
        val execution = async { rig.worker.execute() }
        try {
            rig.send(rig.protocol.invitation(ConnectionManagement.Resource.DISPLAY))
            until { rig.messages().any { it is ConnectionManagement.Prepared } }
            val displayKey = (rig.messages().first { it is ConnectionManagement.Prepared } as ConnectionManagement.Prepared).key
            val display = ConnectionResourceLease(cameraLease().connection, displayKey, 1)
            rig.commit(display); until { rig.presentation.state.value.display != null }
            val shown = rig.presentation.state.value.display
            rig.send(rig.protocol.invitation(ConnectionManagement.Resource.CAMERA, 2))
            until { rig.presentation.state.value.cameraOffer != null }
            val offer = checkNotNull(rig.presentation.state.value.cameraOffer)
            command(rig, rig.presentation.cameraChoice(rig.token, offer, CameraConsentChoice(true, false)))
            until { rig.presentation.state.value.cameraOffer?.offer?.permissionRequired == true }
            val pending = checkNotNull(rig.presentation.state.value.cameraOffer)
            command(rig, rig.presentation.cameraPermission(rig.token, pending, false, CameraConsentChoice(true, false)))
            until { rig.messages().contains(ConnectionManagement.ResultMessage(offer.lease.key, ConnectionManagement.Outcome.PERMISSION_DENIED)) }
            until { rig.presentation.state.value.cameraOffer == null }
            assertEquals(2, rig.opened.get()); assertSame(shown, rig.presentation.state.value.display)
            assertTrue(rig.presentation.state.value.authenticated); assertFalse(execution.isCompleted)
            assertNull(rig.presentation.cameraPermission(rig.token, pending, true, CameraConsentChoice(true, true)))
        } finally {
            rig.worker.cancel(); withTimeout(5000) { execution.await(); rig.worker.retire() }; rig.cleanup()
        }
        assertTrue(rig.retired())
    }
    @Test fun t936PendingPermissionCannotOpenAfterOriginalConnectionCancellation() = runBlocking {
        val rig = ConnectionJoinedCameraFixture()
        rig.send(rig.protocol.invitation(ConnectionManagement.Resource.CAMERA))
        val execution = async { rig.worker.execute() }
        try {
            until { rig.presentation.state.value.cameraOffer != null }
            val offer = checkNotNull(rig.presentation.state.value.cameraOffer)
            command(rig, rig.presentation.cameraChoice(rig.token, offer, CameraConsentChoice(true, false)))
            until { rig.presentation.state.value.cameraOffer?.offer?.permissionRequired == true }
            val pending = checkNotNull(rig.presentation.state.value.cameraOffer)
            rig.worker.cancel(); assertNull(rig.presentation.state.value.cameraOffer)
            assertNull(rig.presentation.cameraPermission(rig.token, pending, true, CameraConsentChoice(true, true)))
            assertFalse(rig.worker.command(ConnectionWorkerCommand.CameraPermission(offer.lease, true, CameraConsentChoice(true, true))))
            assertFalse(rig.worker.command(ConnectionWorkerCommand.CameraConsumerRetired(offer.lease)))
            withTimeout(5000) { execution.await(); rig.worker.retire() }
            assertEquals(0, rig.opened.get()); assertTrue(rig.retired())
        } finally {
            rig.worker.cancel(); withTimeout(5000) { execution.await(); rig.worker.retire() }; rig.cleanup()
        }
    }
}
