package com.blent.connection

import com.blent.MediaConsumerAuthority
import com.blent.MediaConsumerClaim
import com.blent.ResourceRetirement
import kotlinx.coroutines.*
import java.util.Collections
import java.util.IdentityHashMap

/** One exact media lease's consumer requests. Closing revokes new claims;
 * terminal proof additionally requires published closure and actual completion
 * of every claim. Original senders run outside the domain gate. */
internal class ConnectionMediaConsumerOwner(
    private val lease: ConnectionResourceLease,
    commands: ConnectionMediaConsumerCommands,
    send: (ConnectionWorkerCommand) -> Boolean,
    dispatcher: CoroutineDispatcher = Dispatchers.IO,
    delivered: () -> Unit = {},
) : MediaConsumerAuthority {
    private val gate = Any()
    private val claims = Collections.newSetFromMap(IdentityHashMap<Claim, Boolean>())
    private var closing = false
    private var visible = true
    private var gone = false
    private val relay = ConnectionMediaCompletionRelay(commands, lease, send, dispatcher, delivered = delivered)

    private inner class Claim : MediaConsumerClaim {
        override fun current(): Boolean = synchronized(gate) { !closing && !gone && claims.contains(this) }
        override fun retired(proof: ResourceRetirement) {
            synchronized(gate) { claims.remove(this) }
            flush()
        }
    }
    override fun retain(): MediaConsumerClaim? = synchronized(gate) {
        if (closing || gone) null else Claim().also { claims.add(it) }
    }
    fun invalidate() { synchronized(gate) { closing = true } }
    override fun stop(): Boolean {
        synchronized(gate) { closing = true }
        return relay.stop()
    }
    fun closed(lease: ConnectionResourceLease): Boolean {
        synchronized(gate) {
            if (lease != this.lease || gone) return false
            closing = true; visible = false
        }
        flush()
        return true
    }
    private fun flush() {
        val ready = synchronized(gate) { !gone && !visible && claims.isEmpty() }
        if (ready) relay.ready()
    }
    /** Only the original connection's actual worker retirement calls this. */
    suspend fun retire() {
        synchronized(gate) { closing = true; gone = true }
        relay.retire()
    }
}

/** Bounded terminal relay to the original already-created worker. Metadata-only
 * send never binds a replacement or does native I/O. Rejection retains proof and
 * retries one command; native IPC ownership remains in that original worker. */
internal class ConnectionMediaCompletionRelay(
    private val commands: ConnectionMediaConsumerCommands,
    private val lease: ConnectionResourceLease,
    private val send: (ConnectionWorkerCommand) -> Boolean,
    dispatcher: CoroutineDispatcher = Dispatchers.IO,
    private val retryMillis: Long = 20,
    private val delivered: () -> Unit = {},
) {
    private val scope = CoroutineScope(SupervisorJob() + dispatcher)
    private val gate = Any()
    private var job: Job? = null
    private var retired = false
    init {
        require(retryMillis in 1..1000)
        ConnectionLocalLease.validate(lease, commands.resource)
    }
    fun stop(): Boolean {
        if (synchronized(gate) { retired }) return false
        return try {
            send(commands.stop(lease))
        } catch (_: Exception) { false }
    }
    fun ready() {
        val pending = synchronized(gate) {
            if (job != null || retired) return
            scope.launch(start = CoroutineStart.LAZY) { deliver() }.also { job = it }
        }
        pending.start()
    }
    private suspend fun deliver() {
        val command = commands.retired(lease)
        var waitMillis = retryMillis
        while (currentCoroutineContext().isActive) {
            val accepted = try { send(command) } catch (_: Exception) { false }
            if (accepted) {
                synchronized(gate) { retired = true }
                try { delivered() } finally { scope.cancel() }
                return
            }
            delay(waitMillis)
            waitMillis = minOf(1000, waitMillis * 2)
        }
    }
    suspend fun retire() {
        val pending = synchronized(gate) { retired = true; job }
        scope.cancel()
        pending?.join()
    }
}
