package com.blent.connection

import java.util.concurrent.atomic.AtomicReference
import kotlinx.coroutines.*

/** One original private connection shared by independent media fixtures. */
internal class JoinedMediaConnectionFixture {
    val dispatcher = java.util.concurrent.Executors.newSingleThreadExecutor().asCoroutineDispatcher()
    val scope = CoroutineScope(SupervisorJob() + dispatcher)
    private val report = AtomicReference<(ConnectionWorkerEvent) -> Unit>({})
    val link = ConnectionJoinedCameraFixture(15) { report.get()(it) }
    val connection = ConnectionBinding(scope, { _, callback -> report.set(callback); link.worker })
    suspend fun until(condition: () -> Boolean) = withTimeout(4000) { while (!condition()) delay(1) }
    suspend fun connect() {
        withContext(dispatcher) { connection.connect(ConnectionWorkerSpec("joined media")) }
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
    fun retirePeer(lease: ConnectionResourceLease) = synchronized(link.gate) {
        link.protocol.peerClose(lease)
        link.protocol.send(ConnectionManagement.Revoke(lease.key, ConnectionManagement.Reason.REQUESTED))
        link.protocol.pump()
    }
    fun replaceChannel(lease: ConnectionResourceLease) = synchronized(link.gate) {
        for (channel in ConnectionChannels.members(lease.key.resource)) link.protocol.host.replaceChannel(channel, lease.key.generation + 1)
    }
    suspend fun close() {
        withContext(dispatcher) { connection.disconnect() }
        until { connection.state.value.cleanup == null }
        scope.cancel(); link.cleanup(); dispatcher.close()
    }
}
