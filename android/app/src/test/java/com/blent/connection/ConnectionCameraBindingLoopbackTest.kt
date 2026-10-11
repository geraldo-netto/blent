package com.blent.connection

import com.blent.MediaConsumerClaim
import com.blent.ResourceRetirement
import java.net.Socket
import java.util.concurrent.atomic.AtomicReference
import kotlinx.coroutines.*
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [27, 34])
class ConnectionCameraBindingLoopbackTest {
    private suspend fun until(done: () -> Boolean) = withTimeout(4000) { while (!done()) delay(1) }

    @Test fun t948OriginalCaptureClaimsJoinPrivateBinderActorAndRealLoopbackRetirement() = runBlocking {
        val scope = CoroutineScope(SupervisorJob() + Dispatchers.Default)
        val events = AtomicReference<(ConnectionWorkerEvent) -> Unit>({})
        // Robolectric ServiceController creation belongs on its test main thread;
        // production factory remains lazy and the Binder worker starts in execute.
        val rig = ConnectionJoinedCameraFixture { events.get()(it) }
        val binding = ConnectionBinding(scope, { _, report -> events.set(report); rig.worker })
        var socket: Socket? = null
        var first: MediaConsumerClaim? = null
        var second: MediaConsumerClaim? = null
        try {
            binding.connect(ConnectionWorkerSpec("joined camera"))
            until { binding.state.value.authenticated }
            assertNotNull(events.get())
            rig.send(rig.protocol.invitation(ConnectionManagement.Resource.CAMERA))
            until { binding.state.value.cameraOffer != null }
            val offer = checkNotNull(binding.state.value.cameraOffer)
            assertTrue(binding.cameraChoice(offer, CameraConsentChoice(true, false)))
            until { binding.state.value.cameraOffer?.offer?.permissionRequired == true }
            val pending = checkNotNull(binding.state.value.cameraOffer)
            assertTrue(binding.cameraPermission(pending, true, CameraConsentChoice(true, true)))
            until { rig.messages().contains(ConnectionManagement.Prepared(offer.lease.key)) }
            rig.commit(offer.lease)
            until { binding.cameraTarget.value != null }
            val target = checkNotNull(binding.cameraTarget.value)
            first = checkNotNull(target.authority.retain()); second = checkNotNull(target.authority.retain())
            socket = Socket("127.0.0.1", target.endpoint.port)
            val payload = "BLCAM002".toByteArray() + target.endpoint.token.toByteArray() + byteArrayOf(0, 0)
            socket.getOutputStream().write(payload)
            until { rig.payload().size == payload.size }; assertArrayEquals(payload, rig.payload())
            socket.close(); socket = null
            assertTrue(first.current()) // TCP EOF does not prove native capture retirement.
            assertTrue(target.authority.stop())
            until { binding.cameraTarget.value == null }
            assertFalse(first.current()); assertFalse(second.current())
            assertTrue(binding.state.value.authenticated)
            assertFalse(rig.controls.any { it is ConnectionWorkerCommand.CameraConsumerRetired })
            first.retired(ResourceRetirement.NOT_STARTED)
            assertFalse(rig.controls.any { it is ConnectionWorkerCommand.CameraConsumerRetired })
            second.retired(ResourceRetirement.RETIRED)
            until { rig.controls.any { it is ConnectionWorkerCommand.CameraConsumerRetired } }
            assertFalse(binding.cameraPermission(pending, true, CameraConsentChoice(true, true)))
        } finally {
            socket?.close(); first?.retired(ResourceRetirement.RETIRED); second?.retired(ResourceRetirement.RETIRED)
            binding.disconnect()
            until { binding.state.value.cleanup == null }
            assertTrue(rig.retired()); scope.cancel(); rig.cleanup()
        }
    }
}
