package com.blent.connection

import com.blent.*
import kotlinx.coroutines.*
import kotlinx.coroutines.flow.MutableStateFlow
import org.junit.Assert.*

/** Complete consent/consumer path over the private connection and media services. */
internal class ConnectionJoinedAudioFixture(
    val resource: ConnectionManagement.Resource = ConnectionManagement.Resource.SPEAKERS,
    shared: JoinedMediaConnectionFixture? = null,
) {
    private val direction = if (resource == ConnectionManagement.Resource.MICROPHONE) 1 else 2
    @Volatile var permitted = direction == 2
    val prompts = LocalPermissionPrompts()
    val sdk = java.util.concurrent.CopyOnWriteArrayList<LocalPermissionPrompt>()
    private val original = shared ?: JoinedMediaConnectionFixture()
    private val ownsConnection = shared == null
    val dispatcher = original.dispatcher
    val scope = original.scope
    private val audioScope = CoroutineScope(SupervisorJob() + dispatcher)
    val link = original.link
    val connection = original.connection
    val native = JoinedAudioWorkerFixture(direction)
    val binding = AudioBinding(AudioPreferences(processing = 2, background = true, gain = 75, builtIn = false, speakerLowLatency = true), {}, { permitted },
        native::run, MutableStateFlow(null), audioScope, {}, direction, {})
    val access = ConnectionAudioAccess(connection, resource)
    val observation = AudioAccessObservation(binding, access, prompts, audioScope)
    val policy get() = observation.policy
    init { prompts.attach { sdk.add(it); true } }
    suspend fun until(condition: () -> Boolean) = withTimeout(4000) { while (!condition()) delay(1) }
    suspend fun connect() {
        attach(); original.connect()
    }
    suspend fun attach() = withContext(dispatcher) { binding.start(); policy.foreground(true) }
    suspend fun display(): ConnectionDisplayChoice = original.display()
    suspend fun offer(request: Long = 1, background: Boolean = false, generation: Long = 1): AudioAccessOffer {
        val invitation = link.protocol.invitation(resource, request, generation)
        link.send(invitation.copy(profile = (invitation.profile as ConnectionManagement.Audio).copy(background = background)))
        until { policy.state.value.offer != null }
        return checkNotNull(policy.state.value.offer)
    }
    suspend fun start(offer: AudioAccessOffer) = withContext(dispatcher) { policy.start(offer) }
    suspend fun stop(offer: AudioAccessOffer) = withContext(dispatcher) { policy.stop(offer) }
    suspend fun currentAccess() = withContext(dispatcher) { access.project() }
    suspend fun admit(offer: AudioAccessOffer): LocalAudioRoute {
        assertTrue(start(offer))
        return admitPrepared()
    }
    suspend fun admitPrepared(): LocalAudioRoute {
        val lease = checkNotNull(selectedOffer()).lease
        until { link.messages().contains(ConnectionManagement.Prepared(lease.key)) }
        link.commit(lease)
        try { until { selectedRoute()?.lease == lease && native.tasks.isNotEmpty() } }
        catch (failure: TimeoutCancellationException) {
            throw AssertionError("Joined $resource admission: connection=${connection.state.value}; audio=${binding.state.value}; events=${link.events}; cancellations=${link.cancellations}; endpointFailures=${link.nativeEndpoints.map { it.failure() }}; consent=${policy.state.value}; controls=${link.controls}; tasks=${native.tasks.size}; alive=${native.alive}; exit=${native.exitRequested}; messages=${link.messages()}", failure)
        }
        return checkNotNull(selectedRoute()).route
    }
    fun selectedOffer() = if (direction == 1) connection.state.value.microphoneOffer else connection.state.value.speakersOffer
    fun selectedRoute() = if (direction == 1) connection.state.value.microphone else connection.state.value.speakers
    suspend fun approve(granted: Boolean, actual: Boolean = granted): Boolean = withContext(dispatcher) {
        permitted = actual; prompts.result(sdk.last(), granted)
    }
    suspend fun close() {
        native.readRelease.complete(Unit); native.writeRelease.complete(Unit); native.release.complete(Unit); native.die()
        withContext(dispatcher) { observation.retire(); binding.shutdown() }
        if (ownsConnection) original.close()
        audioScope.cancel(); native.close()
    }
}

internal typealias ConnectionJoinedSpeakerFixture = ConnectionJoinedAudioFixture
