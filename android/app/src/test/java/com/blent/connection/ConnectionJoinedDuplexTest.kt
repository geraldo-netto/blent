package com.blent.connection

import com.blent.*
import java.nio.ByteBuffer
import java.nio.ByteOrder
import kotlinx.coroutines.*
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config

private class DuplexFixture {
    val original = JoinedMediaConnectionFixture()
    val microphone = ConnectionJoinedAudioFixture(ConnectionManagement.Resource.MICROPHONE, original)
    val speakers = ConnectionJoinedAudioFixture(ConnectionManagement.Resource.SPEAKERS, original)
    val connection get() = original.connection
    val link get() = original.link
    suspend fun connect(): ConnectionDisplayChoice {
        microphone.connect(); speakers.attach(); return original.display()
    }
    suspend fun admitMicrophone(): LocalAudioRoute {
        val offer = microphone.offer(); assertTrue(microphone.start(offer))
        original.until { microphone.sdk.size == 1 }; assertTrue(microphone.approve(true))
        return microphone.admitPrepared()
    }
    suspend fun admitSpeakers(): LocalAudioRoute = speakers.admit(speakers.offer(request = 3))
    suspend fun playSpeakers() {
        original.until { link.payload(ConnectionWire.SPEAKER).size == 76 }
        assertTrue(link.feedback(duplexGrant(speakers.native.tasks.single().endpoint), ConnectionWire.SPEAKER))
        for (sequence in 0L..3L) { assertTrue(link.feedback(duplexSpeakerPcm(sequence), ConnectionWire.SPEAKER)); delay(10) }
        withTimeout(4000) { speakers.native.audible.await() }
    }
    suspend fun stream() {
        original.until { link.payload(ConnectionWire.MICROPHONE).size >= 76 && link.payload(ConnectionWire.SPEAKER).size >= 76 }
        assertTrue(link.feedback(duplexGrant(microphone.native.tasks.single().endpoint), ConnectionWire.MICROPHONE))
        assertTrue(link.feedback(duplexGrant(speakers.native.tasks.single().endpoint), ConnectionWire.SPEAKER))
        for (sequence in 0L..3L) { assertTrue(link.feedback(duplexSpeakerPcm(sequence), ConnectionWire.SPEAKER)); delay(10) }
        original.until { link.payload(ConnectionWire.MICROPHONE).size >= 1088 && speakers.native.audible.isCompleted }
    }
    suspend fun close() { microphone.close(); speakers.close(); original.close() }
}

private fun duplexGrant(endpoint: AudioEndpoint) = endpoint.token.toByteArray() + ByteBuffer.allocate(92)
    .put("BLAUD002".toByteArray()).put("ab".repeat(32).toByteArray()).putLong(1)
    .put(endpoint.direction.toByte()).put(endpoint.direction.toByte()).putInt(48000).putShort(480)
    .put(endpoint.processing.toByte()).put(if (endpoint.background) 1.toByte() else 0.toByte()).putShort(endpoint.bufferMs.toShort()).array()
private fun duplexSpeakerPcm(sequence: Long): ByteArray {
    val bytes = ByteBuffer.allocate(1972).putLong(1).putLong(sequence).putLong(sequence * 10000)
        .putShort(1944).put(2).put(0).putLong(7).putLong(sequence * 480).putLong(1000000000 + sequence * 10000000)
        .order(ByteOrder.LITTLE_ENDIAN)
    repeat(480) { bytes.putShort(1234).putShort(-4321) }
    return bytes.array()
}

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [27, 34])
class ConnectionJoinedDuplexTest {
    @Test fun t1004RepeatedDuplexAdmissionWithPendingCameraWithdrawalPreservesAudioAuthority() = runBlocking {
        repeat(32) {
            val rig = DuplexFixture()
            try {
                val display = rig.connect()
                val invitation = rig.link.protocol.invitation(ConnectionManagement.Resource.CAMERA, 5)
                rig.link.send(invitation)
                rig.original.until { rig.connection.state.value.cameraOffer != null }
                val microphone = rig.admitMicrophone()
                val speaker = async { rig.admitSpeakers() }
                rig.link.send(ConnectionManagement.Revoke(invitation.key, ConnectionManagement.Reason.REQUESTED))
                val speakers = speaker.await()
                rig.stream()
                rig.original.until { rig.connection.state.value.cameraOffer == null }
                assertSame(display, rig.connection.state.value.display)
                assertEquals(microphone.lease, rig.microphone.selectedRoute()?.lease)
                assertEquals(speakers.lease, rig.speakers.selectedRoute()?.lease)
                assertTrue(rig.microphone.binding.sharing); assertTrue(rig.speakers.binding.sharing)
                assertTrue(rig.connection.state.value.authenticated)
                assertFalse(rig.microphone.native.exitRequested); assertFalse(rig.speakers.native.exitRequested)
            } finally { rig.close() }
        }
    }

    @Test fun t1000BothAudioDirectionsTransferTogetherAndIndependentStopRetainsSiblingAuthority() = runBlocking {
        for (stopMicrophone in listOf(false, true)) {
            val rig = DuplexFixture()
            try {
                val display = rig.connect(); val microphone = rig.admitMicrophone(); val speakers = rig.admitSpeakers()
                rig.stream()
                assertEquals(1, rig.microphone.sdk.size); assertTrue(rig.speakers.sdk.isEmpty())
                val stop = if (stopMicrophone) rig.microphone else rig.speakers
                val sibling = if (stopMicrophone) rig.speakers else rig.microphone
                val retained = checkNotNull(sibling.selectedRoute())
                assertTrue(stop.stop(checkNotNull(stop.policy.state.value.offer))); rig.original.until { stop.native.exitRequested }
                assertNull(stop.selectedRoute()); assertSame(retained, sibling.selectedRoute())
                assertTrue(sibling.binding.sharing); assertSame(display, rig.connection.state.value.display)
                assertFalse(rig.link.controls.any { it is ConnectionWorkerCommand.AudioConsumerRetired })
                stop.native.release.complete(Unit)
                val lease = if (stopMicrophone) microphone.lease else speakers.lease
                rig.original.until { rig.link.controls.contains(ConnectionWorkerCommand.AudioConsumerRetired(lease)) }
                assertSame(retained, sibling.selectedRoute()); assertTrue(rig.connection.state.value.authenticated)
                assertFalse(sibling.native.exitRequested)
            } finally { rig.close() }
        }
    }

    @Test fun t1000DeniedPendingMicrophoneCannotInterruptPlayingSpeakersOrDisplay() = runBlocking {
        val rig = DuplexFixture()
        try {
            val display = rig.connect(); rig.admitSpeakers()
            rig.playSpeakers()
            val retained = checkNotNull(rig.speakers.selectedRoute())
            val offered = rig.microphone.offer(); assertTrue(rig.microphone.start(offered))
            rig.original.until { rig.microphone.sdk.size == 1 }; assertTrue(rig.microphone.approve(false))
            rig.original.until { rig.microphone.policy.state.value.offer == null }
            assertEquals(0, rig.microphone.native.opened.get()); assertTrue(rig.microphone.native.tasks.isEmpty())
            assertSame(retained, rig.speakers.selectedRoute()); assertTrue(rig.speakers.binding.sharing)
            assertSame(display, rig.connection.state.value.display); assertTrue(rig.connection.state.value.authenticated)
            assertFalse(rig.speakers.native.exitRequested)
        } finally { rig.close() }
    }

    @Test fun t1000MicrophoneNativeFailureAndDeathPreserveOriginalPlayingSpeakerAndDisplay() = runBlocking {
        for (openFailure in listOf(false, true)) {
            val rig = DuplexFixture()
            try {
                val display = rig.connect(); rig.admitSpeakers(); rig.playSpeakers()
                val retained = checkNotNull(rig.speakers.selectedRoute())
                if (openFailure) rig.microphone.native.failure = "T1000 synthetic microphone unavailable"
                val offered = rig.microphone.offer(); assertTrue(rig.microphone.start(offered))
                rig.original.until { rig.microphone.sdk.size == 1 }; assertTrue(rig.microphone.approve(true))
                rig.original.until { rig.microphone.selectedOffer() != null && rig.link.messages().any {
                    it is ConnectionManagement.Prepared && it.key.resource == ConnectionManagement.Resource.MICROPHONE
                } }
                val lease = checkNotNull(rig.microphone.selectedOffer()).lease
                rig.link.commit(lease); rig.original.until { rig.microphone.native.tasks.size == 1 }
                if (!openFailure) {
                    rig.original.until { rig.link.payload(ConnectionWire.MICROPHONE).size == 76 }
                    rig.microphone.native.die()
                } else {
                    withTimeout(4000) { rig.microphone.native.failed.await() }
                    assertFalse(rig.microphone.native.cleanup.isCompleted)
                    assertFalse(rig.link.controls.contains(ConnectionWorkerCommand.AudioConsumerRetired(lease)))
                    rig.microphone.native.release.complete(Unit)
                }
                rig.original.until { rig.link.controls.contains(ConnectionWorkerCommand.AudioConsumerRetired(lease)) }
                assertSame(retained, rig.speakers.selectedRoute()); assertTrue(rig.speakers.binding.sharing)
                assertSame(display, rig.connection.state.value.display); assertTrue(rig.connection.state.value.authenticated)
                assertFalse(rig.speakers.native.exitRequested); assertTrue(rig.speakers.native.written.get() > 0)
            } finally { rig.close() }
        }
    }

    @Test fun t1000HeldMicrophoneReadKeepsSpeakerRenderingAndOriginalDisplayControlResponsive() = runBlocking {
        val rig = DuplexFixture()
        try {
            rig.microphone.native.holdRead = true
            val display = rig.connect(); val microphone = rig.admitMicrophone(); rig.admitSpeakers()
            rig.original.until { rig.link.payload(ConnectionWire.MICROPHONE).size == 76 && rig.link.payload(ConnectionWire.SPEAKER).size == 76 }
            assertTrue(rig.link.feedback(duplexGrant(rig.microphone.native.tasks.single().endpoint), ConnectionWire.MICROPHONE))
            assertTrue(rig.link.feedback(duplexGrant(rig.speakers.native.tasks.single().endpoint), ConnectionWire.SPEAKER))
            withTimeout(4000) { rig.microphone.native.captured.await() }
            val retained = checkNotNull(rig.speakers.selectedRoute())
            assertTrue(rig.microphone.stop(checkNotNull(rig.microphone.policy.state.value.offer)))
            rig.original.until { rig.microphone.native.exitRequested }; rig.microphone.native.release.complete(Unit)
            for (sequence in 0L..3L) { assertTrue(rig.link.feedback(duplexSpeakerPcm(sequence), ConnectionWire.SPEAKER)); delay(10) }
            withTimeout(4000) { rig.speakers.native.audible.await() }; delay(50)
            assertFalse(rig.microphone.native.cleanup.isCompleted)
            assertFalse(rig.link.controls.any { it is ConnectionWorkerCommand.AudioConsumerRetired })
            assertSame(retained, rig.speakers.selectedRoute()); assertTrue(rig.speakers.binding.sharing)
            rig.link.send(ConnectionManagement.Revoke(display.route.lease.key, ConnectionManagement.Reason.REQUESTED))
            rig.original.until { rig.connection.state.value.display == null }; assertTrue(rig.connection.state.value.authenticated)
            assertSame(retained, rig.speakers.selectedRoute())
            rig.microphone.native.readRelease.complete(Unit)
            rig.original.until { rig.link.controls.contains(ConnectionWorkerCommand.AudioConsumerRetired(microphone.lease)) }
            assertFalse(rig.speakers.native.exitRequested); assertTrue(rig.speakers.native.written.get() > 0)
        } finally { rig.close() }
    }

    @Test fun t1000CleanMicrophoneReplacementNeedsFreshStartAndDoesNotReplacePlayingSpeaker() = runBlocking {
        val rig = DuplexFixture()
        try {
            val display = rig.connect(); val old = rig.admitMicrophone(); rig.admitSpeakers(); rig.stream()
            val oldConsent = checkNotNull(rig.microphone.policy.state.value.offer)
            val retained = checkNotNull(rig.speakers.selectedRoute())
            assertTrue(rig.microphone.stop(oldConsent)); rig.original.until { rig.microphone.native.exitRequested }
            rig.microphone.native.release.complete(Unit)
            rig.original.until { rig.link.controls.contains(ConnectionWorkerCommand.AudioConsumerRetired(old.lease)) }
            rig.original.retirePeer(old.lease)
            rig.original.until { rig.link.messages().contains(ConnectionManagement.ResultMessage(old.lease.key, ConnectionManagement.Outcome.REVOKED)) }
            rig.original.replaceChannel(old.lease)
            val freshNative = JoinedAudioWorkerFixture(1); rig.microphone.native.next = freshNative
            val offer = rig.microphone.offer(request = 4, generation = 2)
            assertNotSame(oldConsent.owner, offer.owner); assertFalse(rig.microphone.policy.state.value.enabled)
            assertTrue(freshNative.tasks.isEmpty()); assertFalse(rig.microphone.start(oldConsent))
            assertTrue(rig.microphone.start(offer))
            rig.original.until { rig.link.messages().contains(ConnectionManagement.Prepared(checkNotNull(rig.microphone.selectedOffer()).lease.key)) }
            val lease = checkNotNull(rig.microphone.selectedOffer()).lease
            rig.link.commit(lease); rig.original.until { freshNative.tasks.size == 1 && rig.microphone.selectedRoute() != null }
            assertNotEquals(old.lease, lease); assertSame(retained, rig.speakers.selectedRoute())
            assertSame(display, rig.connection.state.value.display); assertTrue(rig.speakers.binding.sharing)
            assertFalse(rig.speakers.native.exitRequested); assertEquals(1, rig.microphone.sdk.size)
        } finally { rig.close() }
    }
}
