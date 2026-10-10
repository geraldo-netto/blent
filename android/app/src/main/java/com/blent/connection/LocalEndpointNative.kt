package com.blent.connection

/** Native adapter owns all partially created handles. Construction is lazy.
 * bind/accept/close may block and run only on retained native workers. close may
 * race bind/accept; late-created objects remain owned and must be closed too. */
internal interface LocalEndpointNative : ConnectionSetupLifetime {
    fun bind(): Int
    fun accept(): ConnectionIo
}

/** Prompt, serialized ownership contract; implementations retain native work.
 * Factories must retain uncertain partial construction if they throw. */
internal interface LocalEndpoint {
    fun advance(now: Long)
    fun admit(now: Long)
    fun port(): Int?
    fun takeIo(): ConnectionIo?
    fun stop(now: Long)
    fun failure(): Exception?
    fun retired(): Boolean
    fun quarantined(): Boolean
}

/** Prompt per-channel native supervision. Binding runs off the connection actor;
 * accept may wait idle until admission, then has its own absolute startup budget.
 * Stop is logical; ownership remains until both native workers and backend retire.
 * A timed-out or failed close never frees a sibling's or replacement's admission. */
internal class LocalEndpointOwner(
    private val native: LocalEndpointNative,
    private val startedAt: Long,
    private val limits: ConnectionRunLimits = ConnectionRunLimits(),
    private val wake: () -> Unit = {},
    private val threads: (String, Runnable) -> Thread = { name, work -> Thread(work, name).apply { isDaemon = true } },
) : LocalEndpoint {
    private val gate = Object()
    private var previous = startedAt
    private var bound: Int? = null
    private var ready: ConnectionIo? = null
    private var accepted = false
    private var admittedAt: Long? = null
    private var stoppedAt: Long? = null
    private var error: Exception? = null
    private var closeError: Exception? = null
    private var worker: Thread? = null
    private var closer: Thread? = null
    private var startFinished = false
    private var closeFinished = false

    init { time(startedAt); start() }
    private fun time(now: Long) {
        require(now >= 0 && now >= previous && now <= Long.MAX_VALUE - maxOf(limits.ioTimeoutMs, limits.retirementTimeoutMs))
        previous = now
    }
    private fun start() {
        try {
            closer = threads("blent-local-close", Runnable { closeNative() })
            checkNotNull(closer).start()
            worker = threads("blent-local-open", Runnable { openNative() })
            checkNotNull(worker).start()
        } catch (failure: Exception) { synchronized(gate) { fail(failure) } }
        finally { synchronized(gate) { startFinished = true }; signal() }
    }
    private fun signal() {
        try { wake() } catch (failure: Exception) { synchronized(gate) { fail(failure) } }
    }
    private fun fail(failure: Exception) {
        if (error == null) error = failure
        stopLocked()
    }
    private fun stopLocked() {
        if (stoppedAt != null) return
        stoppedAt = previous
        bound = null; ready = null
        gate.notifyAll()
    }
    private fun openNative() {
        try {
            synchronized(gate) { check(stoppedAt == null) { "Local endpoint cancelled" } }
            val port = native.bind()
            require(port in 1..65535) { "Invalid local listener port" }
            synchronized(gate) {
                check(stoppedAt == null) { "Local bind completed after cancellation" }
                bound = port
            }
            signal()
            val io = native.accept()
            synchronized(gate) {
                check(stoppedAt == null) { "Local accept completed after cancellation" }
                ready = io; accepted = true
            }
        } catch (failure: Exception) { synchronized(gate) { fail(failure) } }
        finally { signal() }
    }
    private fun closeNative() {
        try {
            synchronized(gate) { while (stoppedAt == null) gate.wait() }
            native.close()
        } catch (failure: Exception) { synchronized(gate) { closeError = failure; fail(failure) } }
        finally { synchronized(gate) { closeFinished = true }; signal() }
    }
    override fun advance(now: Long) = synchronized(gate) {
        time(now)
        if (stoppedAt != null) return@synchronized
        val origin = if (bound == null) startedAt else admittedAt ?: return@synchronized
        if (!accepted && now - origin >= limits.ioTimeoutMs) fail(IllegalStateException("Local endpoint startup deadline expired"))
    }
    override fun admit(now: Long) = synchronized(gate) {
        time(now)
        check(stoppedAt == null && bound != null && admittedAt == null)
        admittedAt = now
    }
    override fun port(): Int? = synchronized(gate) { bound }
    override fun takeIo(): ConnectionIo? = synchronized(gate) { ready.also { ready = null } }
    override fun stop(now: Long) { synchronized(gate) { time(now); stopLocked() }; signal() }
    override fun failure(): Exception? = synchronized(gate) { error ?: closeError }
    override fun retired(): Boolean = synchronized(gate) {
        startFinished && stoppedAt != null && closeFinished && closeError == null &&
            worker?.isAlive != true && closer?.isAlive != true && native.retired()
    }
    override fun quarantined(): Boolean = synchronized(gate) {
        val stopped = stoppedAt ?: return@synchronized false
        !retired() && (closeError != null || previous - stopped >= limits.retirementTimeoutMs)
    }
}
