package com.blent.connection

import com.blent.CameraCaptureTarget
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers

/** Camera metadata adapter to the original worker/resource session owner. */
internal class ConnectionCameraSession(
    send: (ConnectionWorkerCommand) -> Boolean,
    dispatcher: CoroutineDispatcher = Dispatchers.IO,
) {
    private val session = ConnectionMediaConsumerSession<LocalCameraRoute, CameraCaptureTarget>(
        ConnectionCameraConsumerCommands, send, dispatcher,
    ) { route, authority -> CameraCaptureTarget(ConnectionCameraTarget.endpoint(route), authority) }
    fun admitted(route: LocalCameraRoute): CameraCaptureTarget? = session.admitted(route)
    fun closed(lease: ConnectionResourceLease): Boolean = session.closed(lease)
    fun clear() = session.clear()
    fun stop() = session.stop()
    suspend fun retire() = session.retire()
}
