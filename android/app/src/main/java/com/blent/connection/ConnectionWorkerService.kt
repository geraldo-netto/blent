package com.blent.connection

import android.app.Service
import android.content.Intent
import android.os.Binder
import android.os.IBinder
import android.os.Parcel
import android.os.Process
import kotlinx.coroutines.*
import kotlinx.coroutines.channels.Channel
import java.util.concurrent.atomic.AtomicBoolean

/** Private process adapter; Binder handles bounded metadata, never native work.
 * Every instance exits its process on destruction; one START is admitted in
 * that lifetime. Factory setup remains T854 and is explicitly unsupported here. */
open class ConnectionWorkerService : Service() {
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.IO)
    private val admitted = AtomicBoolean()
    private val stopped = AtomicBoolean()
    private val cancelSent = AtomicBoolean()
    private val commands = Channel<ConnectionWorkerCommand>(32)
    private val events = Channel<ConnectionWorkerEvent>(32)
    private var generation = 0L
    @Volatile private var worker: ConnectionWorker? = null
    @Volatile private var callback: IBinder? = null
    private var execution: Job? = null
    private var controls: Job? = null
    private val clientDeath = IBinder.DeathRecipient { terminate() }

    internal open fun createWorker(spec: ConnectionWorkerSpec, report: (ConnectionWorkerEvent) -> Unit): ConnectionWorker =
        throw UnsupportedOperationException("Production USB setup is not implemented (T854)")
    internal open fun terminate() { Process.killProcess(Process.myPid()) }

    private val binder = object : Binder() {
        override fun onTransact(code: Int, data: Parcel, reply: Parcel?, flags: Int): Boolean {
            try {
                ConnectionWorkerIpc.validate(data, ConnectionWorkerIpc.REQUEST, flags)
                return command(code, ConnectionWorkerIpc.generation(data), data)
            } catch (_: Exception) { terminate(); return false }
        }
    }
    @Synchronized private fun command(code: Int, revision: Long, data: Parcel): Boolean {
        if (revision != generation) return false
        when (code) {
            ConnectionWorkerIpc.START -> start(data)
            ConnectionWorkerIpc.CONTROL -> control(data)
            ConnectionWorkerIpc.STOP -> { ConnectionWorkerIpc.end(data); stop() }
            ConnectionWorkerIpc.EXIT -> { ConnectionWorkerIpc.end(data); terminate() }
            else -> return false
        }
        return true
    }
    private fun start(data: Parcel) {
        val spec = ConnectionWorkerCodec.spec(ConnectionWorkerIpc.bytes(data))
        val remote = requireNotNull(data.readStrongBinder())
        ConnectionWorkerIpc.end(data)
        check(!stopped.get() && admitted.compareAndSet(false, true))
        callback = remote
        execution = scope.launch { execute(spec, remote) }
    }
    private fun control(data: Parcel) {
        val value = ConnectionWorkerCodec.command(ConnectionWorkerIpc.bytes(data))
        ConnectionWorkerIpc.end(data)
        check(admitted.get() && !stopped.get() && commands.trySend(value).isSuccess)
    }
    private suspend fun execute(spec: ConnectionWorkerSpec, remote: IBinder) {
        var failure: Exception? = null
        try {
            link(remote)
            scope.launch { deliver(remote) }
            val owned = createWorker(spec, ::publish)
            worker = owned
            if (!stopped.get()) {
                controls = scope.launch { controls(owned) }
                owned.execute()
            }
        } catch (error: Exception) { if (error !is CancellationException) failure = error }
        finally {
            controls?.cancel()
            withContext(NonCancellable) { finish(failure) }
        }
    }
    private fun link(remote: IBinder) {
        try { remote.linkToDeath(clientDeath, 0) }
        catch (error: Exception) { terminate(); throw error }
    }
    private suspend fun controls(owned: ConnectionWorker) {
        try {
            for (value in commands) {
                if (stopped.get()) return
                check(owned.command(value)) { "Connection control backlog exceeded" }
            }
        } catch (error: Exception) { if (error !is CancellationException) terminate() }
    }
    private suspend fun deliver(remote: IBinder) {
        try { for (event in events) ConnectionWorkerIpc.event(remote, generation, event) }
        catch (error: Exception) { if (error !is CancellationException) terminate() }
    }
    private fun publish(event: ConnectionWorkerEvent) {
        try {
            ConnectionWorkerCodec.event(event)
            check(events.trySend(event).isSuccess) { "Connection event backlog exceeded" }
        } catch (_: Exception) { terminate() }
    }
    private fun cancelNative(owned: ConnectionWorker) {
        if (cancelSent.compareAndSet(false, true)) owned.cancel()
    }
    private suspend fun finish(original: Exception?) {
        var failure = original
        worker?.let { owned ->
            try { cancelNative(owned) } catch (error: Exception) { if (failure == null) failure = error }
            try { owned.retire() } catch (error: Exception) { if (failure == null) failure = error }
        }
        publish(ConnectionWorkerEvent.Ended(failure?.message?.replace('\u0000', '?')?.take(512)))
    }
    private fun stop() {
        if (!stopped.compareAndSet(false, true)) return
        commands.close()
        execution?.cancel()
        scope.launch {
            try { worker?.let(::cancelNative) }
            catch (_: Exception) { terminate() }
        }
    }
    @Synchronized override fun onBind(intent: Intent): IBinder {
        val revision = intent.getLongExtra("connection_generation", 0)
        require(revision > 0)
        check(generation == 0L || generation == revision) { "Connection process already owned" }
        generation = revision
        return binder
    }
    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int) = START_NOT_STICKY
    override fun onUnbind(intent: Intent): Boolean { terminate(); return false }
    override fun onDestroy() {
        try { callback?.unlinkToDeath(clientDeath, 0) }
        finally {
            scope.cancel(); commands.close(); events.close()
            try { terminate() } finally { super.onDestroy() }
        }
    }
}
