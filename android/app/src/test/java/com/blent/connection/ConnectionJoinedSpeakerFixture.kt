package com.blent.connection

import com.blent.*
import java.util.concurrent.atomic.AtomicReference
import kotlinx.coroutines.*
import kotlinx.coroutines.flow.MutableStateFlow
import org.junit.Assert.*

/** Complete consent/consumer path over the private connection and media services. */
internal class ConnectionJoinedAudioFixture(val resource: ConnectionManagement.Resource = ConnectionManagement.Resource.SPEAKERS) {
    private val direction = if (resource == ConnectionManagement.Resource.MICROPHONE) 1 else 2
    @Volatile var permitted = direction == 2
    val prompts = LocalPermissionPrompts()
    val sdk = java.util.concurrent.CopyOnWriteArrayList<LocalPermissionPrompt>()
    val dispatcher = java.util.concurrent.Executors.newSingleThreadExecutor().asCoroutineDispatcher()
    val scope = CoroutineScope(SupervisorJob() + dispatcher)
    private val audioScope = CoroutineScope(SupervisorJob() + dispatcher)
    private val report = AtomicReference<(ConnectionWorkerEvent) -> Unit>({})
    val link = ConnectionJoinedCameraFixture(15) { report.get()(it) }
    val connection = ConnectionBinding(scope, { _, callback -> report.set(callback); link.worker })
    val native = JoinedAudioWorkerFixture(direction)
    val binding = AudioBinding(AudioPreferences(processing = 2, background = true, gain = 75, builtIn = false, speakerLowLatency = true), {}, { permitted },
        native::run, MutableStateFlow(null), audioScope, {}, direction, {})
    val access = ConnectionAudioAccess(connection, resource)
    val observation = AudioAccessObservation(binding, access, prompts, audioScope)
    val policy get() = observation.policy
    init { prompts.attach { sdk.add(it); true } }
    suspend fun until(condition: () -> Boolean) = withTimeout(4000) { while (!condition()) delay(1) }
    suspend fun connect() {
        withContext(dispatcher) {
            binding.start(); policy.foreground(true)
            connection.connect(ConnectionWorkerSpec("joined speaker"))
        }
        until { connection.state.value.authenticated }
    }
    suspend fun display(): ConnectionDisplayChoice {
        link.send(link.protocol.invitation(ConnectionManagement.Resource.DISPLAY, 2))
        until { link.messages().any { it is ConnectionManagement.Prepared && it.key.resource == ConnectionManagement.Resource.DISPLAY } }
        val key = (link.messages().first { it is ConnectionManagement.Prepared && it.key.resource == ConnectionManagement.Resource.DISPLAY } as ConnectionManagement.Prepared).key
        link.commit(ConnectionResourceLease(cameraLease().connection, key, 1))
        until { connection.state.value.display != null }
        return checkNotNull(connection.state.value.display)
    }
    suspend fun offer(request: Long = 1, background: Boolean = false): AudioAccessOffer {
        val invitation = link.protocol.invitation(resource, request)
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
        until { link.messages().any { it is ConnectionManagement.Prepared && it.key.resource == resource } }
        val lease = checkNotNull(selectedOffer()).lease
        link.commit(lease); until { selectedRoute() != null && native.tasks.isNotEmpty() }
        return checkNotNull(selectedRoute()).route
    }
    fun selectedOffer() = if (direction == 1) connection.state.value.microphoneOffer else connection.state.value.speakersOffer
    fun selectedRoute() = if (direction == 1) connection.state.value.microphone else connection.state.value.speakers
    suspend fun approve(granted: Boolean, actual: Boolean = granted): Boolean = withContext(dispatcher) {
        permitted = actual; prompts.result(sdk.last(), granted)
    }
    suspend fun close() {
        native.readRelease.complete(Unit); native.writeRelease.complete(Unit); native.release.complete(Unit); native.die()
        withContext(dispatcher) { observation.retire(); binding.shutdown(); connection.disconnect() }
        until { connection.state.value.cleanup == null }; scope.cancel(); audioScope.cancel(); native.close(); link.cleanup(); dispatcher.close()
    }
}

internal typealias ConnectionJoinedSpeakerFixture = ConnectionJoinedAudioFixture
