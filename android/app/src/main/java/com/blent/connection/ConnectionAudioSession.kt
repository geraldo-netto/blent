package com.blent.connection

import com.blent.AudioConsumerTarget
import com.blent.AudioEndpoint
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers

/** Typed audio boundary. Playback/capture sees validated endpoint settings and
 * exact consumer authority; wire/worker implementation stays in this adapter. */
internal object ConnectionAudioTarget {
    fun endpoint(route: LocalAudioRoute): AudioEndpoint {
        val token = route.credential.bytes().joinToString("") { "%02x".format(it.toInt() and 255) }
        val direction = if (route.lease.key.resource == ConnectionManagement.Resource.MICROPHONE) 1 else 2
        return AudioEndpoint(token, route.port, direction, route.profile.processing, route.profile.bufferMs, route.profile.background)
    }
}
internal class ConnectionAudioSession(
    resource: ConnectionManagement.Resource,
    send: (ConnectionWorkerCommand) -> Boolean,
    dispatcher: CoroutineDispatcher = Dispatchers.IO,
) {
    private val session = ConnectionMediaConsumerSession<LocalAudioRoute, AudioConsumerTarget>(
        ConnectionAudioConsumerCommands(resource), send, dispatcher,
    ) { route, authority -> AudioConsumerTarget(ConnectionAudioTarget.endpoint(route), authority) }
    fun admitted(route: LocalAudioRoute): AudioConsumerTarget? = session.admitted(route)
    fun closed(lease: ConnectionResourceLease): Boolean = session.closed(lease)
    fun clear() = session.clear()
    fun stop() = session.stop()
    suspend fun retire() = session.retire()
}
