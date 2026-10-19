package com.blent.connection

import java.util.ArrayDeque

/** One native permission worker, one independent closer, bounded callback queue.
 * The main owner polls advance independently. No stop/callback performs native I/O.
 * Detach invalidates selection synchronously, before waking presentation/replacement. */
internal class AccessoryAdmissionOwner(
    private val backend: AccessoryAdmissionBackend,
    profile: AccessoryProfile,
    capacity: Int = 32,
    private val clock: () -> Long,
    private val nativeTimeoutMs: Long = 10000,
    private val permissionTimeoutMs: Long = 120000,
    private val retirementTimeoutMs: Long = 5000,
    private val wake: () -> Unit = {},
    private val threads: (String, Runnable) -> Thread = { name, body -> Thread(body, name).apply { isDaemon = true } },
) : AccessoryAdmission, ConnectionDeadlineSource {
    private val gate = Object()
    private val selection = AccessorySelectionOwner(profile, capacity)
    private val commands = ArrayDeque<() -> Unit>()
    private var ready: AccessorySelection? = null
    private var accepted = false
    private var stopped = false
    private var error: Exception? = null
    private var closeError: Exception? = null
    private var worker: Thread? = null
    private var closer: Thread? = null
    private var started = false
    private var closed = false
    private var lastTime = clock()
    private var deadline = 0L
    private var nativeDeadline = Long.MAX_VALUE
    private var retirementDeadline = Long.MAX_VALUE

    init {
        require(nativeTimeoutMs in 1..60000 && retirementTimeoutMs in 1..60000)
        require(permissionTimeoutMs in 1..600000)
        require(lastTime in 0..Long.MAX_VALUE - 600000)
        deadline = lastTime + nativeTimeoutMs
        start()
    }
    private fun now(): Long = clock().also {
        require(it in lastTime..Long.MAX_VALUE - 600000) { "Invalid admission clock" }
        lastTime = it
    }
    private fun signal() {
        try { wake() } catch (failure: Exception) { synchronized(gate) { failLocked(failure) } }
    }
    private fun failLocked(failure: Exception) { if (error == null) error = failure; stopLocked() }
    private fun fail(failure: Exception) { synchronized(gate) { failLocked(failure) }; signal() }
    private fun live() { check(!stopped) { "Accessory admission stopped" }; check(now() < minOf(deadline, nativeDeadline)) { "Accessory admission deadline" } }
    private fun stopLocked() {
        if (stopped) return
        stopped = true; selection.invalidate(); ready = null; commands.clear()
        val time = try { now() } catch (failure: Exception) { if (error == null) error = failure; lastTime }
        retirementDeadline = time + retirementTimeoutMs
        gate.notifyAll()
    }
    override fun stop() { synchronized(gate) { stopLocked() }; signal() }
    private fun dispatch(command: () -> Unit) {
        synchronized(gate) {
            if (stopped) return
            if (commands.size == 32) { failLocked(IllegalStateException("Accessory callback backlog")); return }
            commands.addLast(command); gate.notifyAll()
        }
    }
    private fun start() {
        try {
            closer = threads("blent-permission-close", Runnable { closeNative() }); checkNotNull(closer).start()
            worker = threads("blent-permission", Runnable { runNative() }); checkNotNull(worker).start()
        } catch (failure: Exception) { fail(failure) }
        finally { synchronized(gate) { started = true }; signal() }
    }
    private fun runNative() {
        try {
            synchronized(gate) { live() }
            backend.start(::dispatch, ::confirm, ::stop)
            select()
            while (true) {
                val command = synchronized(gate) {
                    while (!stopped && commands.isEmpty()) gate.wait()
                    if (stopped) null else commands.removeFirst()
                } ?: return
                nativeCall(command)
            }
        } catch (failure: Exception) { fail(failure) }
        finally { signal() }
    }
    private fun select() {
        synchronized(gate) { live() }
        val snapshot = backend.snapshot()
        val chosen = synchronized(gate) { live(); selection.select(snapshot) }
        val permitted = backend.permitted(chosen)
        synchronized(gate) { live() }
        if (permitted) confirm(chosen, backend.snapshot(), true)
        else {
            synchronized(gate) { live(); deadline = now() + permissionTimeoutMs }
            nativeCall { backend.request(chosen) }
        }
    }
    private fun nativeCall(action: () -> Unit) {
        synchronized(gate) { live(); nativeDeadline = minOf(deadline, now() + nativeTimeoutMs) }
        action()
        synchronized(gate) { live(); nativeDeadline = Long.MAX_VALUE }
    }
    private fun confirm(value: AccessorySelection, snapshot: List<AccessoryDescription>, permitted: Boolean) {
        try {
            synchronized(gate) {
                if (stopped || accepted || !selection.current(value)) return
                live()
                checkNotNull(selection.confirm(value, snapshot, permitted)) { "Accessory permission denied or device changed" }
                accepted = true; ready = value; deadline = Long.MAX_VALUE
            }
        } catch (failure: Exception) { fail(failure) }
        signal()
    }
    private fun closeNative() {
        try {
            synchronized(gate) { while (!stopped) gate.wait() }
            backend.close()
        } catch (failure: Exception) { synchronized(gate) { closeError = failure } }
        finally { synchronized(gate) { closed = true }; signal() }
    }
    override fun advance() {
        try {
            val failure = backend.failure()
            synchronized(gate) {
                val time = now()
                if (failure != null) failLocked(failure)
                if (!stopped && time >= minOf(deadline, nativeDeadline)) failLocked(IllegalStateException("Accessory admission deadline"))
            }
        } catch (failure: Exception) { fail(failure) }
    }
    override fun takeReady(): AccessorySelection? = synchronized(gate) { ready.also { ready = null } }
    override fun current(selection: AccessorySelection): Boolean = synchronized(gate) { !stopped && this.selection.current(selection) }
    override fun failure(): Exception? = synchronized(gate) { error ?: closeError }
    override fun retired(): Boolean = synchronized(gate) {
        started && stopped && closed && worker?.isAlive != true && closer?.isAlive != true && backend.retired()
    }
    override fun terminationRequired(): Boolean = synchronized(gate) {
        closeError != null || (stopped && lastTime >= retirementDeadline && !retired())
    }
    override fun nextDeadline(): ConnectionDeadline = synchronized(gate) {
        when {
            !stopped -> ConnectionDeadline.at(minOf(deadline, nativeDeadline))
            closeError != null -> ConnectionDeadline.IDLE
            lastTime >= retirementDeadline -> ConnectionDeadline.IDLE
            retired() -> ConnectionDeadline.IDLE
            else -> ConnectionDeadline.at(retirementDeadline)
        }
    }
}
