package com.blent.connection

import com.blent.*
import java.nio.ByteBuffer
import kotlinx.coroutines.*
import okio.Buffer
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [27, 34])
class ConnectionJoinedMicrophoneTest {
    private fun fixture() = ConnectionJoinedAudioFixture(ConnectionManagement.Resource.MICROPHONE)
    private fun grant(endpoint: AudioEndpoint, clocked: Boolean = true) = endpoint.token.toByteArray() + ByteBuffer.allocate(92)
        .put((if (clocked) "BLAUD002" else "BLAUD001").toByteArray()).put("ab".repeat(32).toByteArray()).putLong(1)
        .put(1).put(1).putInt(48000).putShort(480).put(endpoint.processing.toByte())
        .put(if (endpoint.background) 1.toByte() else 0.toByte()).putShort(endpoint.bufferMs.toShort()).array()
    private suspend fun permitted(rig: ConnectionJoinedAudioFixture): LocalAudioRoute {
        consent(rig)
        return rig.admitPrepared()
    }
    private suspend fun consent(rig: ConnectionJoinedAudioFixture) {
        val offer = rig.offer(); assertTrue(rig.start(offer)); rig.until { rig.sdk.size == 1 }
        assertEquals(0, rig.native.opened.get()); assertEquals(0, rig.link.opened.get() - if (rig.connection.state.value.display != null) 2 else 0)
        assertTrue(rig.approve(true))
    }
    @Test fun t998OriginalSdkConsentJoinsPrivateServicesClockedPcmAndActualWorkerExit() = runBlocking {
        val rig = fixture()
        try {
            rig.connect(); val route = permitted(rig)
            rig.until { rig.link.payload(ConnectionWire.MICROPHONE).size == 76 }
            val endpoint = rig.native.tasks.single().endpoint
            val greeting = rig.link.payload(ConnectionWire.MICROPHONE)
            assertEquals("BLAUREQ2", String(greeting, 0, 8)); assertEquals(endpoint.token, String(greeting, 8, 64))
            assertArrayEquals(byteArrayOf(7, 0, 1, 2), greeting.copyOfRange(72, 76))
            assertEquals(40, endpoint.bufferMs); assertEquals(75, rig.native.tasks.single().preferences.gain)
            assertFalse(rig.native.tasks.single().preferences.builtIn); assertEquals(0, rig.native.started.get())
            assertTrue(rig.link.feedback(grant(endpoint), ConnectionWire.MICROPHONE))
            rig.until { rig.link.payload(ConnectionWire.MICROPHONE).size >= 76 + 2024 }
            val pcm = Buffer().write(rig.link.payload(ConnectionWire.MICROPHONE).copyOfRange(76, 76 + 2024))
            val packets = AudioPackets(1, 1, true)
            val first = packets.read(pcm); val second = packets.read(pcm)
            assertEquals(480, first.samples.size); assertTrue(first.samples.all { it == 925.toShort() })
            assertEquals(7L, checkNotNull(first.clock).epoch)
            assertTrue(checkNotNull(second.clock).frames > checkNotNull(first.clock).frames)
            assertEquals(1, rig.native.started.get())
            assertTrue(rig.stop(checkNotNull(rig.policy.state.value.offer)))
            rig.until { rig.connection.microphoneTarget.value == null && rig.native.exitRequested }
            assertFalse(rig.link.controls.any { it is ConnectionWorkerCommand.AudioConsumerRetired })
            assertEquals(0, rig.native.unbound.get()); assertFalse(rig.native.cleanup.isCompleted)
            rig.native.release.complete(Unit)
            rig.until { rig.link.controls.contains(ConnectionWorkerCommand.AudioConsumerRetired(route.lease)) }
            assertEquals(1, rig.native.closes.get()); assertEquals(1, rig.native.unbound.get())
            assertTrue(rig.connection.state.value.authenticated); assertFalse(rig.binding.sharing)
        } finally { rig.close() }
    }

    @Test fun t998DeniedAndUnprovenSdkGrantNeverOpenCaptureAndDisplayRemainsCurrent() = runBlocking {
        for (granted in listOf(false, true)) {
            val rig = fixture()
            try {
                rig.connect(); val display = rig.display(); val offer = rig.offer(); assertTrue(rig.start(offer))
                rig.until { rig.sdk.size == 1 }; assertTrue(rig.approve(granted, false))
                rig.until { rig.policy.state.value.offer == null }
                assertEquals(0, rig.native.opened.get()); assertTrue(rig.native.tasks.isEmpty())
                assertSame(display, rig.connection.state.value.display); assertTrue(rig.connection.state.value.authenticated)
                assertFalse(rig.policy.state.value.enabled); assertFalse(rig.start(offer))
            } finally { rig.close() }
        }
    }

    @Test fun t998PendingStopCancelsOriginalSdkReplyAndKeepsDisplayIndependent() = runBlocking {
        val rig = fixture()
        try {
            rig.connect(); val display = rig.display(); val offer = rig.offer(); assertTrue(rig.start(offer))
            rig.until { rig.sdk.size == 1 }; val prompt = rig.sdk.single()
            assertTrue(rig.stop(checkNotNull(rig.policy.state.value.offer)))
            rig.until { rig.policy.state.value.offer == null }
            withContext(rig.dispatcher) { rig.permitted = true; assertFalse(rig.prompts.result(prompt, true)) }
            assertEquals(0, rig.native.opened.get()); assertEquals(2, rig.link.opened.get())
            assertSame(display, rig.connection.state.value.display); assertFalse(rig.start(offer))
        } finally { rig.close() }
    }

    @Test fun t998NativeOpenFailureAndWorkerDeathWithdrawOnlyOriginalMicrophone() = runBlocking {
        for (openFailure in listOf(false, true)) {
            val rig = fixture()
            try {
                rig.connect(); val display = rig.display()
                if (openFailure) { rig.native.failure = "synthetic microphone unavailable"; rig.native.release.complete(Unit) }
                val lease = if (openFailure) failedCapture(rig) else permitted(rig).lease.also {
                    rig.until { rig.link.payload(ConnectionWire.MICROPHONE).size == 76 }; rig.native.die()
                }
                rig.until { rig.link.controls.contains(ConnectionWorkerCommand.AudioConsumerRetired(lease)) }
                assertFalse(rig.binding.sharing); assertNull(rig.connection.microphoneTarget.value)
                assertSame(display, rig.connection.state.value.display); assertTrue(rig.connection.state.value.authenticated)
                assertNull(rig.connection.state.value.speakers)
            } finally { rig.close() }
        }
    }
    private suspend fun failedCapture(rig: ConnectionJoinedAudioFixture): ConnectionResourceLease {
        consent(rig)
        rig.until { rig.link.messages().any { it is ConnectionManagement.Prepared && it.key.resource == rig.resource } }
        val lease = checkNotNull(rig.selectedOffer()).lease
        rig.link.commit(lease)
        return lease
    }

    @Test fun t998HeldNativeReadNeverRetiresConsumerOrBlocksDisplayControl() = runBlocking {
        val rig = fixture()
        try {
            rig.native.holdRead = true
            rig.connect(); val display = rig.display(); val route = permitted(rig)
            rig.until { rig.link.payload(ConnectionWire.MICROPHONE).size == 76 }
            assertTrue(rig.link.feedback(grant(rig.native.tasks.single().endpoint), ConnectionWire.MICROPHONE))
            withTimeout(4000) { rig.native.captured.await() }
            assertTrue(rig.stop(checkNotNull(rig.policy.state.value.offer))); rig.until { rig.native.exitRequested }
            rig.native.release.complete(Unit); delay(50)
            assertFalse(rig.native.cleanup.isCompleted); assertFalse(rig.link.controls.any { it is ConnectionWorkerCommand.AudioConsumerRetired })
            assertEquals(0, rig.native.unbound.get()); assertSame(display, rig.connection.state.value.display)
            rig.link.send(ConnectionManagement.Revoke(display.route.lease.key, ConnectionManagement.Reason.REQUESTED))
            rig.until { rig.connection.state.value.display == null }; assertTrue(rig.connection.state.value.authenticated)
            rig.native.readRelease.complete(Unit)
            rig.until { rig.link.controls.contains(ConnectionWorkerCommand.AudioConsumerRetired(route.lease)) }
            assertEquals(1, rig.native.closes.get())
        } finally { rig.close() }
    }

    @Test fun t998FreshPermissionLossWithdrawsCaptureButActualWorkerDeathRemainsNecessary() = runBlocking {
        val rig = fixture()
        try {
            rig.native.acceptExit = false
            rig.connect(); val display = rig.display(); val route = permitted(rig)
            rig.until { rig.link.payload(ConnectionWire.MICROPHONE).size == 76 }
            assertTrue(rig.link.feedback(grant(rig.native.tasks.single().endpoint, false), ConnectionWire.MICROPHONE))
            rig.until { rig.link.payload(ConnectionWire.MICROPHONE).size >= 76 + 988 }
            val block = AudioPackets(1, 1).read(Buffer().write(rig.link.payload(ConnectionWire.MICROPHONE).copyOfRange(76, 76 + 988)))
            assertNull(block.clock); assertEquals(480, block.samples.size)
            withContext(rig.dispatcher) { rig.permitted = false; rig.policy.permissionChanged() }
            rig.until { rig.connection.microphoneTarget.value == null && rig.native.exitRequested }
            rig.native.release.complete(Unit); withTimeout(2000) { rig.native.cleanup.await() }; delay(50)
            assertFalse(rig.link.controls.any { it is ConnectionWorkerCommand.AudioConsumerRetired })
            assertSame(display, rig.connection.state.value.display); assertEquals(0, rig.native.unbound.get())
            rig.native.die(); rig.until { rig.link.controls.contains(ConnectionWorkerCommand.AudioConsumerRetired(route.lease)) }
            assertTrue(rig.connection.state.value.authenticated)
        } finally { rig.close() }
    }

    @Test fun t998OriginalConnectionLossInvalidatesPendingSdkResultWithoutNativeCapture() = runBlocking {
        val rig = fixture()
        try {
            rig.connect(); val offer = rig.offer(); assertTrue(rig.start(offer)); rig.until { rig.sdk.size == 1 }
            rig.connection.disconnect(); rig.until { !rig.connection.state.value.active && rig.policy.state.value.offer == null }
            assertFalse(rig.approve(true)); assertFalse(rig.start(offer))
            rig.until { rig.connection.state.value.cleanup == null }
            assertEquals(0, rig.native.opened.get()); assertTrue(rig.native.tasks.isEmpty())
            assertFalse(rig.link.controls.any { it is ConnectionWorkerCommand.AudioConsumerRetired })
        } finally { rig.close() }
    }

    @Test fun t998BackgroundCaptureNeedsHostAndLocalConsentAndForegroundLossCancelsPendingSdk() = runBlocking {
        val rig = fixture()
        try {
            rig.connect(); val display = rig.display(); val offer = rig.offer(background = true)
            assertTrue(rig.start(offer)); rig.until { rig.sdk.size == 1 }; assertTrue(rig.approve(true))
            val route = rig.admitPrepared(); rig.until { rig.link.payload(ConnectionWire.MICROPHONE).size == 76 }
            assertTrue(rig.native.tasks.single().endpoint.background)
            withContext(rig.dispatcher) {
                assertTrue(rig.policy.background(checkNotNull(rig.policy.state.value.offer), true))
                rig.policy.foreground(false)
            }
            assertTrue(rig.binding.sharing); assertNotNull(rig.connection.microphoneTarget.value)
            withContext(rig.dispatcher) { assertTrue(rig.policy.background(checkNotNull(rig.policy.state.value.offer), false)) }
            rig.until { rig.native.exitRequested && rig.connection.microphoneTarget.value == null }
            assertSame(display, rig.connection.state.value.display)
            rig.native.release.complete(Unit)
            rig.until { rig.link.controls.contains(ConnectionWorkerCommand.AudioConsumerRetired(route.lease)) }
        } finally { rig.close() }
        val pending = fixture()
        try {
            pending.connect(); val offer = pending.offer(); assertTrue(pending.start(offer)); pending.until { pending.sdk.size == 1 }
            withContext(pending.dispatcher) { pending.policy.foreground(false) }
            pending.until { pending.policy.state.value.offer == null }
            assertFalse(pending.approve(true)); assertEquals(0, pending.native.opened.get())
        } finally { pending.close() }
    }
}
