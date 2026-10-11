package com.blent.connection

import com.blent.*
import java.util.concurrent.atomic.AtomicReference
import kotlinx.coroutines.*
import kotlinx.coroutines.flow.MutableStateFlow
import org.junit.Assert.*

/** Complete consent/consumer path over the private connection and media services. */
internal class ConnectionJoinedSpeakerFixture {
    val dispatcher = java.util.concurrent.Executors.newSingleThreadExecutor().asCoroutineDispatcher()
    val scope = CoroutineScope(SupervisorJob() + dispatcher)
    private val audioScope = CoroutineScope(SupervisorJob() + dispatcher)
    private val report = AtomicReference<(ConnectionWorkerEvent) -> Unit>({})
    val link = ConnectionJoinedCameraFixture(11) { report.get()(it) }
    val connection = ConnectionBinding(scope, { _, callback -> report.set(callback); link.worker })
    val native = JoinedSpeakerWorkerFixture()
    val binding = AudioBinding(AudioPreferences(processing = 2, background = true, gain = 75, builtIn = false, speakerLowLatency = true), {}, { true },
        native::run, MutableStateFlow(null), audioScope, {}, 2, {})
    val access = ConnectionAudioAccess(connection, ConnectionManagement.Resource.SPEAKERS)
    val observation = AudioAccessObservation(binding, access, LocalPermissionPrompts(), audioScope)
    val policy get() = observation.policy
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
    suspend fun offer(request: Long = 1): AudioAccessOffer {
        link.send(link.protocol.invitation(ConnectionManagement.Resource.SPEAKERS, request))
        until { policy.state.value.offer != null }
        return checkNotNull(policy.state.value.offer)
    }
    suspend fun start(offer: AudioAccessOffer) = withContext(dispatcher) { policy.start(offer) }
    suspend fun stop(offer: AudioAccessOffer) = withContext(dispatcher) { policy.stop(offer) }
    suspend fun currentAccess() = withContext(dispatcher) { access.project() }
    suspend fun admit(offer: AudioAccessOffer): LocalAudioRoute {
        assertTrue(start(offer))
        until { link.messages().any { it is ConnectionManagement.Prepared && it.key.resource == ConnectionManagement.Resource.SPEAKERS } }
        val lease = checkNotNull(connection.state.value.speakersOffer).lease
        link.commit(lease); until { connection.state.value.speakers != null && native.tasks.isNotEmpty() }
        return checkNotNull(connection.state.value.speakers).route
    }
    suspend fun close() {
        native.writeRelease.complete(Unit); native.release.complete(Unit); native.die()
        withContext(dispatcher) { observation.retire(); binding.shutdown(); connection.disconnect() }
        until { connection.state.value.cleanup == null }; scope.cancel(); audioScope.cancel(); native.close(); link.cleanup(); dispatcher.close()
    }
}
