package com.blent.connection

import com.blent.*
import kotlinx.coroutines.*
import org.junit.Assert.*

/** T1245: UI borrows controls on the original serialized owner dispatcher.
 * Real Binder/TCP and audio protocol; synthetic device/SDK/death witnesses. */
internal class JoinedAudioUiFixture {
    class Controls(private val audio: ConnectionJoinedAudioFixture) : MediaAccessControls<AudioAccessOffer> {
        override val state get() = audio.policy.state
        override fun start(offer: AudioAccessOffer) = runBlocking(audio.dispatcher) { audio.policy.start(offer) }
        override fun stop(offer: AudioAccessOffer) = runBlocking(audio.dispatcher) { audio.policy.stop(offer) }
        override fun background(offer: AudioAccessOffer, allowed: Boolean) =
            runBlocking(audio.dispatcher) { audio.policy.background(offer, allowed) }
    }

    val original = JoinedMediaConnectionFixture()
    val microphone = ConnectionJoinedAudioFixture(ConnectionManagement.Resource.MICROPHONE, original)
    val speakers = ConnectionJoinedAudioFixture(ConnectionManagement.Resource.SPEAKERS, original)
    val microphoneControls = Controls(microphone)
    val speakerControls = Controls(speakers)

    suspend fun connect(): ConnectionDisplayChoice {
        microphone.connect(); speakers.attach()
        return original.display()
    }

    suspend fun offerBoth() {
        microphone.offer(); speakers.offer(request = 3)
    }

    suspend fun play(native: JoinedAudioWorkerFixture = speakers.native, bootstrapBytes: Int = 76) {
        original.until { original.link.payload(ConnectionWire.SPEAKER).size >= bootstrapBytes }
        assertTrue(original.link.feedback(duplexGrant(native.tasks.single().endpoint), ConnectionWire.SPEAKER))
        feedSpeakers(0)
        withTimeout(4000) { native.audible.await() }
    }

    suspend fun feedSpeakers(first: Long) {
        for (sequence in first..first + 3) {
            assertTrue(original.link.feedback(duplexSpeakerPcm(sequence), ConnectionWire.SPEAKER))
            delay(10)
        }
    }

    suspend fun capture() {
        original.until { original.link.payload(ConnectionWire.MICROPHONE).size == 76 }
        val endpoint = microphone.native.tasks.single().endpoint
        assertTrue(original.link.feedback(duplexGrant(endpoint), ConnectionWire.MICROPHONE))
        withTimeout(4000) { microphone.native.captured.await() }
    }

    fun terminal(lease: ConnectionResourceLease) =
        original.link.controls.contains(ConnectionWorkerCommand.AudioConsumerRetired(lease))

    suspend fun retire(audio: ConnectionJoinedAudioFixture, route: LocalAudioRoute) {
        original.until { audio.native.exitRequested }
        assertFalse(terminal(route.lease))
        audio.native.release.complete(Unit)
        original.until { terminal(route.lease) }
        assertTrue(audio.native.cleanup.isCompleted)
        assertEquals(1, audio.native.unbound.get())
        assertEquals(1, audio.native.closes.get())
    }

    suspend fun close() {
        try { microphone.close() }
        finally {
            try { speakers.close() }
            finally { original.close() }
        }
    }
}
