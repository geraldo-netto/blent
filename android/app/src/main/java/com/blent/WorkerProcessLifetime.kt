package com.blent

import android.content.ComponentName
import android.content.Context
import android.content.Intent
import android.content.ServiceConnection
import android.os.IBinder
import kotlinx.coroutines.*
import kotlinx.coroutines.selects.select

/** T1706: sole owner of a bound worker's native lifetime. Protocol adapters own
 * their messages and original transmit jobs, and supply their joins at retirement.
 * Ended allows EXIT; only original Binder death plus every native call/link return
 * allows unbind/unlink. Timeout retains the same owner and cleanup jobs for retry. */
internal class WorkerProcessLifetime(
    context: Context,
    private val scope: CoroutineScope,
    private val graceMillis: Long,
    private val disconnected: () -> Unit,
    private val rejected: () -> Unit,
    private val linkFailed: (Exception, IBinder) -> Unit,
    private val stopCommand: (IBinder) -> Unit,
    private val exitCommand: (IBinder) -> Unit,
) {
    private val app = context.applicationContext
    val gate = Any()
    val ready = CompletableDeferred<IBinder>()
    val dead = CompletableDeferred<Unit>()
    val ended = CompletableDeferred<Unit>()
    private val stopIssued = CompletableDeferred<Unit>()
    @Volatile var remote: IBinder? = null
        private set
    @Volatile var startExposed = false
    @Volatile var retired = false
        private set
    @Volatile private var bound = false
    private var acceptingBinding = true
    private var linking: CompletableDeferred<Unit>? = null
    private var stopping: Job? = null
    private var stopCall: Job? = null
    private var exitCall: Job? = null
    private var cleanup: Deferred<Unit>? = null
    private val death = IBinder.DeathRecipient { dead.complete(Unit); disconnected() }
    private val connection = object : ServiceConnection {
        override fun onServiceConnected(name: ComponentName?, service: IBinder?) {
            if (service == null) { rejected(); return }
            connected(service)
        }
        override fun onServiceDisconnected(name: ComponentName?) = disconnected()
        override fun onBindingDied(name: ComponentName?) = disconnected()
        override fun onNullBinding(name: ComponentName?) = rejected()
    }

    private fun connected(service: IBinder) {
        val settled = synchronized(gate) {
            if (!bound || !acceptingBinding || remote != null) return
            CompletableDeferred<Unit>().also { linking = it; remote = service }
        }
        try { service.linkToDeath(death, 0); ready.complete(service) }
        catch (error: Exception) { linkFailed(error, service) }
        finally { settled.complete(Unit) }
    }

    fun bind(intent: Intent, flags: Int) {
        bound = true
        try { check(app.bindService(intent, connection, flags)) { "Cannot bind worker" } }
        catch (error: Exception) { bound = false; throw error }
    }

    fun observeDeath() {
        if (remote?.isBinderAlive == false) dead.complete(Unit)
    }

    fun stop() = synchronized(gate) {
        if (stopping == null) stopping = scope.launch { stopAndExit() }
    }

    private suspend fun stopAndExit() {
        val service = try { ready.await() } catch (_: Exception) { remote ?: return }
        // Separate jobs preserve the exit timer even when native STOP blocks.
        stopCall = scope.launch(Dispatchers.IO) { runCatching { stopCommand(service) } }
        stopIssued.complete(Unit)
        withTimeoutOrNull(graceMillis) { select<Unit> { dead.onAwait {}; ended.onAwait {} } }
        if (!dead.isCompleted) {
            exitCall = scope.launch(Dispatchers.IO) { runCatching { exitCommand(service) } }
        }
    }

    suspend fun awaitStopCall() {
        stopIssued.await()
        stopCall?.join()
    }

    suspend fun retire(waitMillis: Long, original: suspend () -> Unit, drain: suspend () -> Unit = {}) {
        if (retired) return
        withTimeout(waitMillis) {
            original()
            if (requiresDeath()) awaitDeath() else stopping?.cancel()
            drain()
            stopping?.join(); stopCall?.join(); exitCall?.join()
            synchronized(gate) { linking }?.await()
            clean().await()
        }
        retired = true
        scope.cancel()
    }

    private fun requiresDeath(): Boolean = synchronized(gate) {
        if (startExposed || remote != null) true
        else { acceptingBinding = false; false }
    }

    private suspend fun awaitDeath() {
        while (!dead.isCompleted) {
            observeDeath()
            if (!dead.isCompleted) delay(5)
        }
    }

    private fun clean(): Deferred<Unit> = synchronized(gate) {
        cleanup ?: scope.async<Unit>(Dispatchers.IO) {
            if (bound) { app.unbindService(connection); bound = false }
            remote?.unlinkToDeath(death, 0)
        }.also { cleanup = it }
    }
}
