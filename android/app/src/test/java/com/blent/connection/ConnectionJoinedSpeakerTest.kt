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

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [27, 34])
class ConnectionJoinedSpeakerTest {
    private fun grant(endpoint: AudioEndpoint) = endpoint.token.toByteArray() + ByteBuffer.allocate(92)
        .put("BLAUD002".toByteArray()).put("ab".repeat(32).toByteArray()).putLong(1)
        .put(2).put(2).putInt(48000).putShort(480).put(endpoint.processing.toByte())
        .put(if (endpoint.background) 1.toByte() else 0.toByte()).putShort(endpoint.bufferMs.toShort()).array()
    private fun pcm(sequence: Long): ByteArray {
        val packet = ByteBuffer.allocate(1972).putLong(1).putLong(sequence).putLong(sequence * 10000)
            .putShort(1944).put(2).put(0).putLong(7).putLong(sequence * 480).putLong(1000000000 + sequence * 10000000)
            .order(ByteOrder.LITTLE_ENDIAN)
        repeat(480) { packet.putShort(1234).putShort(-4321) }
        return packet.array()
    }
    @Test fun t978LocalConsentJoinsPrivateServicesClockedPlaybackAndActualWorkerExit() = runBlocking {
        val rig = ConnectionJoinedSpeakerFixture()
        try {
            rig.connect(); val offer = rig.offer(); assertEquals(0, rig.link.opened.get()); assertEquals(0, rig.native.opened.get())
            val route = rig.admit(offer); val endpoint = rig.native.tasks.single().endpoint
            rig.until { rig.link.payload(ConnectionWire.SPEAKER).size == 76 }
            val greeting = rig.link.payload(ConnectionWire.SPEAKER)
            assertEquals("BLAUREQ2", String(greeting, 0, 8)); assertEquals(endpoint.token, String(greeting, 8, 64))
            assertArrayEquals(byteArrayOf(7, 0, 2, 2), greeting.copyOfRange(72, 76))
            assertEquals(40, endpoint.bufferMs); assertEquals(75, rig.native.tasks.single().preferences.gain)
            assertTrue(rig.native.tasks.single().preferences.speakerLowLatency)
            assertTrue(rig.link.feedback(grant(endpoint), ConnectionWire.SPEAKER))
            for (sequence in 0L..3L) { assertTrue(rig.link.feedback(pcm(sequence), ConnectionWire.SPEAKER)); delay(10) }
            withTimeout(4000) { rig.native.audible.await() }
            assertTrue(rig.stop(checkNotNull(rig.policy.state.value.offer)))
            rig.until { rig.connection.speakersTarget.value == null && rig.native.exitRequested }
            assertFalse(rig.binding.sharing); assertTrue(rig.connection.state.value.authenticated)
            assertFalse(rig.link.controls.any { it is ConnectionWorkerCommand.AudioConsumerRetired })
            assertEquals(0, rig.native.unbound.get()); assertFalse(rig.native.cleanup.isCompleted)
            rig.native.release.complete(Unit)
            rig.until { rig.link.controls.contains(ConnectionWorkerCommand.AudioConsumerRetired(route.lease)) }
            assertEquals(1, rig.native.closes.get()); assertEquals(1, rig.native.unbound.get())
            assertFalse(rig.currentAccess()?.authority?.current() == true)
        } finally { rig.close() }
    }
    @Test fun t978DeniedOrCancelledPendingOfferNeverOpensAudioAndDisplayStaysIndependent() = runBlocking {
        val rig = ConnectionJoinedSpeakerFixture()
        try {
            rig.connect()
            val display = rig.display()
            val offer = rig.offer(); assertTrue(rig.stop(offer))
            rig.until { rig.policy.state.value.offer == null }
            assertSame(display, rig.connection.state.value.display); assertTrue(rig.connection.state.value.authenticated)
            assertEquals(0, rig.native.opened.get()); assertEquals(2, rig.link.opened.get())
            assertFalse(rig.start(offer)); assertTrue(rig.native.tasks.isEmpty())
        } finally { rig.close() }
    }
    @Test fun t978OriginalPlaybackWorkerDeathWithdrawsOnlySpeakerAndRetainsCleanupBarrier() = runBlocking {
        val rig = ConnectionJoinedSpeakerFixture()
        try {
            rig.connect(); val display = rig.display(); val offer = rig.offer(); val route = rig.admit(offer)
            rig.until { rig.link.payload(ConnectionWire.SPEAKER).size == 76 }
            val endpoint = rig.native.tasks.single().endpoint
            assertTrue(rig.link.feedback(grant(endpoint), ConnectionWire.SPEAKER))
            rig.native.die()
            assertSame(display, rig.connection.state.value.display)
            rig.until { !rig.binding.sharing && rig.connection.speakersTarget.value == null }
            assertTrue(rig.connection.state.value.authenticated); assertNull(rig.connection.state.value.microphone)
            rig.until { rig.link.controls.contains(ConnectionWorkerCommand.AudioConsumerRetired(route.lease)) }
            assertFalse(rig.start(offer))
        } finally { rig.close() }
    }
    @Test fun t978PendingConsentCannotAcquireAfterConnectionStopOrOldPublication() = runBlocking {
        val rig = ConnectionJoinedSpeakerFixture()
        try {
            rig.connect(); val offer = rig.offer()
            rig.connection.disconnect(); rig.until { !rig.connection.state.value.active }
            assertFalse(rig.start(offer)); assertEquals(0, rig.native.opened.get())
            assertTrue(rig.native.tasks.isEmpty()); rig.until { rig.connection.state.value.cleanup == null }
            assertFalse(rig.link.controls.any { it is ConnectionWorkerCommand.AudioConsumerRetired })
        } finally { rig.close() }
    }
    @Test fun t978NativeOpenFailureClosesSpeakerWithoutLosingOriginalConnection() = runBlocking {
        val rig = ConnectionJoinedSpeakerFixture()
        try {
            rig.native.failure = "synthetic speaker unavailable"; rig.native.release.complete(Unit)
            rig.connect(); val offer = rig.offer(); assertTrue(rig.start(offer))
            rig.until { rig.link.messages().any { it is ConnectionManagement.Prepared } }
            val lease = checkNotNull(rig.connection.state.value.speakersOffer).lease
            rig.link.commit(lease)
            rig.until { rig.link.controls.contains(ConnectionWorkerCommand.AudioConsumerRetired(lease)) }
            assertFalse(rig.binding.sharing); assertNull(rig.connection.speakersTarget.value)
            assertEquals(0, rig.native.opened.get()); assertTrue(rig.connection.state.value.authenticated)
            assertFalse(rig.start(offer))
        } finally { rig.close() }
    }
    @Test fun t978EndedNativePlaybackStillCannotRetireBeforeActualPrivateWorkerDeath() = runBlocking {
        val rig = ConnectionJoinedSpeakerFixture()
        try {
            rig.native.acceptExit = false
            rig.connect(); val offer = rig.offer(); val route = rig.admit(offer)
            rig.until { rig.link.payload(ConnectionWire.SPEAKER).size == 76 }
            assertTrue(rig.stop(checkNotNull(rig.policy.state.value.offer)))
            rig.until { rig.native.exitRequested }; rig.native.release.complete(Unit)
            withTimeout(2000) { rig.native.cleanup.await() }
            delay(50)
            assertFalse(rig.link.controls.any { it is ConnectionWorkerCommand.AudioConsumerRetired })
            assertEquals(0, rig.native.unbound.get()); assertTrue(rig.connection.state.value.authenticated)
            assertFalse(rig.start(offer))
            rig.native.die()
            rig.until { rig.link.controls.contains(ConnectionWorkerCommand.AudioConsumerRetired(route.lease)) }
            assertEquals(1, rig.native.unbound.get())
        } finally { rig.close() }
    }

    @Test fun t978BlockedNativeRendererDoesNotBlockDisplayControlOrInventConsumerRetirement() = runBlocking {
        val rig = ConnectionJoinedSpeakerFixture()
        try {
            rig.native.holdWrite = true
            rig.connect(); val display = rig.display(); val route = rig.admit(rig.offer())
            rig.until { rig.link.payload(ConnectionWire.SPEAKER).size == 76 }
            val endpoint = rig.native.tasks.single().endpoint
            assertTrue(rig.link.feedback(grant(endpoint), ConnectionWire.SPEAKER))
            for (sequence in 0L..3L) { assertTrue(rig.link.feedback(pcm(sequence), ConnectionWire.SPEAKER)); delay(10) }
            withTimeout(4000) { rig.native.audible.await() }
            assertTrue(rig.stop(checkNotNull(rig.policy.state.value.offer)))
            rig.until { rig.native.exitRequested && rig.connection.speakersTarget.value == null }
            assertSame(display, rig.connection.state.value.display); assertTrue(rig.connection.state.value.authenticated)
            rig.native.release.complete(Unit); delay(50)
            assertFalse(rig.native.cleanup.isCompleted)
            assertFalse(rig.link.controls.any { it is ConnectionWorkerCommand.AudioConsumerRetired })
            assertEquals(0, rig.native.unbound.get())
            // Independent display control still advances while the speaker native write is held.
            rig.link.send(ConnectionManagement.Revoke(display.route.lease.key, ConnectionManagement.Reason.REQUESTED))
            rig.until { rig.connection.state.value.display == null }
            assertTrue(rig.connection.state.value.authenticated)
            rig.native.writeRelease.complete(Unit)
            rig.until { rig.link.controls.contains(ConnectionWorkerCommand.AudioConsumerRetired(route.lease)) }
            assertNull(rig.connection.state.value.display)
        } finally { rig.close() }
    }

}
