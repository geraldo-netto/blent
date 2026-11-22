package com.blent.connection

import java.util.concurrent.ArrayBlockingQueue
import java.util.concurrent.CountDownLatch
import java.util.concurrent.Executors
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicLong
import java.util.concurrent.atomic.AtomicReference
import kotlinx.coroutines.*

/** Hosted actor measurements only: synthetic idle setup/permission ownership.
 * One outstanding cue and one receipt; the admitted finite cue limit bounds each owner. */
internal class Probe(private val cueLimit: Int) {
    init { require(cueLimit in 1..20000) }
    private data class Cue(val identity: Long, val sent: Long)
    private data class Receipt(val identity: Long, val latency: Long)
    private val pending = AtomicReference<Cue>()
    private val receipts = ArrayBlockingQueue<Receipt>(1)
    private var identity = 0L
    val advances = AtomicLong()
    val started = CountDownLatch(1)
    val wake = AtomicReference<() -> Unit>()
    fun advance() {
        advances.incrementAndGet()
        val cue = pending.getAndSet(null)
        if (cue != null) check(receipts.offer(Receipt(cue.identity, System.nanoTime() - cue.sent)))
        started.countDown()
    }
    fun change(hinted: Boolean, checkOwner: () -> Unit): Long {
        check(identity < cueLimit)
        val cue = Cue(++identity, System.nanoTime())
        check(pending.compareAndSet(null, cue))
        if (hinted) checkNotNull(wake.get()).invoke()
        while (true) {
            val receipt = receipts.poll(1, TimeUnit.SECONDS)
            if (receipt != null) {
                check(receipt.identity == cue.identity)
                return receipt.latency
            }
            checkOwner()
            println("T918 waiting for original cue ${cue.identity}; completed advances=${advances.get()}")
        }
    }
}

private fun worker(spec: ConnectionWorkerSpec, probe: Probe, known: Boolean, clock: () -> Long): ConnectionWorker {
    val state = RuntimeSetup()
    val source = object : ConnectionSetup by state, ConnectionDeadlineSource {
        override fun advance() { probe.advance() }
        override fun stop() { state.stop(); state.gone = true }
        override fun nextDeadline(): ConnectionDeadline = ConnectionDeadline.IDLE
    }
    val owner: ConnectionSetup = if (known) source else object : ConnectionSetup by source {}
    return AndroidConnectionWorker(spec, { _, wake -> probe.wake.set(wake); owner }, {}, clock)
}

private fun admission(spec: ConnectionWorkerSpec, probe: Probe, known: Boolean, clock: () -> Long, dispatcher: CoroutineDispatcher): ConnectionWorker {
    val state = RunAdmission()
    val source = object : AccessoryAdmission by state, ConnectionDeadlineSource {
        override fun advance() { probe.advance() }
        override fun nextDeadline(): ConnectionDeadline = ConnectionDeadline.IDLE
    }
    val owner: AccessoryAdmission = if (known) source else object : AccessoryAdmission by source {}
    return ConnectionAdmissionRun(spec, { wake -> probe.wake.set(wake); owner },
        { _, _ -> error("T918 synthetic permission owner must not admit a worker") }, {}, dispatcher = dispatcher, clock = clock)
}

internal class ActorOwner(val role: String, fallback: Long, known: Boolean, clock: () -> Long, cueLimit: Int) : AutoCloseable {
    val probe = Probe(cueLimit)
    private val original = AtomicReference<Thread>()
    private val executor = Executors.newSingleThreadExecutor { work ->
        Thread(work, "blent-idle-$role").also { check(original.compareAndSet(null, it)) }
    }
    private val dispatcher = executor.asCoroutineDispatcher()
    private val scope = CoroutineScope(SupervisorJob() + dispatcher)
    private val spec = ConnectionWorkerSpec("benchmark", timerMs = fallback)
    private val actor = when (role) {
        "worker" -> worker(spec, probe, known, clock)
        "admission" -> admission(spec, probe, known, clock, dispatcher)
        else -> error("Unknown benchmark role")
    }
    private val execution = scope.async { actor.execute() }
    fun thread(): Thread = checkNotNull(original.get())
    fun checkOwner() {
        if (execution.isCompleted) runBlocking { execution.await(); error("T918 actor exited during funded measurement") }
    }
    fun start() {
        while (!probe.started.await(1, TimeUnit.SECONDS)) {
            checkOwner()
            println("T918 awaiting $role original first advance")
        }
    }
    override fun close() {
        actor.cancel()
        try { runBlocking { try { execution.await() } finally { actor.retire() } } }
        finally { scope.cancel(); dispatcher.close(); original.get()?.join() }
    }
}

