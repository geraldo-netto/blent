package com.blent.connection

/** Native requests borrow exact framing receipts; metadata never copies payloads. */
internal sealed interface ConnectionIoRequest {
    data class Read(val loan: ConnectionBuffer) : ConnectionIoRequest
    data class Write(val loan: ConnectionBuffer) : ConnectionIoRequest
    data class Flush(val loan: ConnectionFlush) : ConnectionIoRequest
}
internal data class ConnectionIoCompletion(
    val request: ConnectionIoRequest, val count: Int, val failure: Exception? = null,
)

/** All methods return promptly. One read and one write/flush may be outstanding,
 * including unconsumed completions. Native code runs only in owned workers.
 * stop revokes admission; retired proves read, write and close actually exited.
 * A timeout or successful stop call alone cannot prove retirement. */
internal interface ConnectionIo {
    fun submit(request: ConnectionIoRequest): Boolean
    fun poll(): ConnectionIoCompletion?
    fun stop()
    fun retired(): Boolean
    fun closeFailure(): Exception?
}
