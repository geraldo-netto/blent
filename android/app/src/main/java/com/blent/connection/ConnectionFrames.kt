package com.blent.connection

/** T881 transport contracts. Adapters serialize callbacks through one owner.
 * A loan authorizes one native operation only; its exact object must accompany
 * completion. Cancellation never establishes completion or permits buffer reuse.
 * These types grant neither authenticated identity nor resource authority. */
internal class ConnectionBuffer internal constructor(val bytes: ByteArray, val offset: Int, val length: Int)
internal class ConnectionFlush internal constructor()

/** The caller transfers the selected array range until actual frame completion.
 * Native adapters and consumers must not retain it after acknowledging release. */
internal data class ConnectionFrame(
    val header: ConnectionWire.Header, val bytes: ByteArray, val offset: Int, val length: Int,
)

/** Absolute active-I/O budget. Idle time and completed-frame dispatch are not
 * charged. The native supervisor must also observe this deadline while calls
 * block; a cooperative callback alone cannot bound a stuck syscall. */
internal class ConnectionFrameClock(private val timeoutMs: Long) : ConnectionDeadlineSource {
    private var previous = 0L
    private var deadline: Long? = null
    var failed = false
        private set
    init { require(timeoutMs in 1..60_000) }

    fun check(now: Long) {
        check(!failed) { "Frame transport closed" }
        try {
            require(now >= previous) { "Invalid monotonic time" }
            previous = now
            check(deadline?.let { now < it } != false) { "Frame I/O deadline expired" }
        } catch (error: Exception) { failed = true; throw error }
    }
    fun start(now: Long) {
        check(now)
        require(now <= Long.MAX_VALUE - timeoutMs)
        check(deadline == null)
        deadline = now + timeoutMs
    }
    fun clear() { deadline = null }
    fun cancel() { failed = true }
    override fun nextDeadline(): ConnectionDeadline =
        if (failed) ConnectionDeadline.IDLE else ConnectionDeadline.at(deadline)
}
