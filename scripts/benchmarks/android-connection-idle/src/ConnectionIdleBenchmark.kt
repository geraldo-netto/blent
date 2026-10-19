package com.blent.connection

import java.nio.file.Files
import java.nio.file.Path
import java.util.concurrent.ArrayBlockingQueue
import java.util.concurrent.CountDownLatch
import java.util.concurrent.Executors
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicLong
import java.util.concurrent.atomic.AtomicReference
import kotlinx.coroutines.*
import kotlinx.serialization.json.*

/** Hosted actor measurements only: synthetic idle setup/permission ownership.
 * One outstanding cue, one receipt, at most 64 cues per owner. */
private class Probe {
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
        check(identity < 64)
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

private class ActorOwner(val role: String, fallback: Long, known: Boolean, clock: () -> Long) : AutoCloseable {
    val probe = Probe()
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

internal data class Counters(val cpu: Long, val allocated: Long?)
/** Hosted JDK API adapter; Android compilation deliberately keeps its normal boot classpath. */
internal class NativeCounters {
    private val factory = Class.forName("java.lang.management.ManagementFactory")
    private val threadContract = Class.forName("com.sun.management.ThreadMXBean")
    private val processContract = Class.forName("com.sun.management.OperatingSystemMXBean")
    private val bean = factory.getMethod("getThreadMXBean").invoke(null)
    private val process = factory.getMethod("getOperatingSystemMXBean").invoke(null)
    private val cpu = threadContract.getMethod("getThreadCpuTime", java.lang.Long.TYPE)
    private val allocated = threadContract.getMethod("getThreadAllocatedBytes", java.lang.Long.TYPE)
    private val processCpu = processContract.getMethod("getProcessCpuTime")
    private val allocationSupported = flag("isThreadAllocatedMemorySupported")
    init {
        check(flag("isThreadCpuTimeSupported")) { "T918 thread CPU counters unavailable" }
        enable("setThreadCpuTimeEnabled")
        if (allocationSupported) enable("setThreadAllocatedMemoryEnabled")
    }
    private fun flag(name: String): Boolean = threadContract.getMethod(name).invoke(bean) as Boolean
    private fun enable(name: String) {
        threadContract.getMethod(name, java.lang.Boolean.TYPE).invoke(bean, true)
    }
    fun read(thread: Thread): Counters {
        val time = cpu.invoke(bean, thread.id) as Long
        check(time >= 0) { "T918 original thread CPU counter unavailable" }
        val bytes = if (allocationSupported) allocated.invoke(bean, thread.id) as Long else null
        check(bytes == null || bytes >= 0)
        return Counters(time, bytes)
    }
    fun processCpu(): Long = (processCpu.invoke(process) as Long).also {
        check(it >= 0) { "T918 JVM process CPU counter unavailable" }
    }
}

private data class RoleResult(
    val role: String, val elapsed: Long, val advances: Long, val cpu: Long, val allocated: Long?,
    val hinted: List<Long>, val missed: List<Long>,
)

private fun changes(owner: ActorOwner, hinted: Boolean, count: Int): List<Long> {
    require(count in 1..32)
    return List(count) {
        owner.checkOwner()
        val latency = owner.probe.change(hinted, owner::checkOwner)
        Thread.sleep(3)
        latency
    }
}

private fun finishResult(owner: ActorOwner, before: Counters, after: Counters, advances: Long, elapsed: Long): RoleResult {
    val allocated = before.allocated?.let { first -> after.allocated?.let { it - first } }
    return RoleResult(owner.role, elapsed, advances, after.cpu - before.cpu, allocated,
        changes(owner, true, 32), changes(owner, false, 2))
}

private fun roleJson(value: RoleResult): JsonObject = buildJsonObject {
    put("role", value.role); put("idle_elapsed_ns", value.elapsed); put("idle_advances", value.advances)
    put("idle_actor_cpu_ns", value.cpu)
    value.allocated?.let { put("idle_actor_allocated_bytes", it) }
    putJsonArray("hinted_latency_ns") { value.hinted.forEach { add(it) } }
    putJsonArray("missed_hint_latency_ns") { value.missed.forEach { add(it) } }
}

private fun trial(fallback: Long, known: Boolean, counters: NativeCounters, clock: () -> Long): JsonObject {
    ActorOwner("worker", fallback, known, clock).use { worker ->
        ActorOwner("admission", fallback, known, clock).use { admission ->
            worker.start(); admission.start(); Thread.sleep(1000)
            val workerBefore = counters.read(worker.thread()); val admissionBefore = counters.read(admission.thread())
            val workerTicks = worker.probe.advances.get(); val admissionTicks = admission.probe.advances.get()
            val processBefore = counters.processCpu()
            val began = System.nanoTime()
            Thread.sleep(3000)
            val elapsed = System.nanoTime() - began
            val processCpu = counters.processCpu() - processBefore
            val workerAfter = counters.read(worker.thread()); val admissionAfter = counters.read(admission.thread())
            val workerAdvances = worker.probe.advances.get() - workerTicks
            val admissionAdvances = admission.probe.advances.get() - admissionTicks
            val workerResult = finishResult(worker, workerBefore, workerAfter, workerAdvances, elapsed)
            val admissionResult = finishResult(admission, admissionBefore, admissionAfter, admissionAdvances, elapsed)
            println("T918 settled fallback=$fallback known=$known: worker idle advances=${workerResult.advances}, admission idle advances=${admissionResult.advances}")
            return buildJsonObject {
                put("fallback_ms", fallback); put("known", known)
                put("idle_process_cpu_ns", processCpu)
                putJsonArray("roles") { add(roleJson(workerResult)); add(roleJson(admissionResult)) }
            }
        }
    }
}

fun main(arguments: Array<String>) {
    require(arguments.size == 1)
    val counters = NativeCounters()
    val origin = System.nanoTime()
    val clock = { (System.nanoTime() - origin) / 1_000_000 }
    val results = arrayListOf<JsonObject>()
    for (fallback in listOf(10L, 100L, 1000L, 1000L, 100L, 10L)) {
        println("T918 starting known fallback=$fallback trial=${results.size + 1}/7")
        results.add(trial(fallback, true, counters, clock))
    }
    println("T918 starting unsupported-capability fallback control")
    results.add(trial(1000, false, counters, clock))
    val report = buildJsonObject {
        put("scope", "Hosted JVM actor scheduling; synthetic idle native ownership; no Android hardware")
        put("java", System.getProperty("java.runtime.version")); put("vm", System.getProperty("java.vm.name"))
        put("os", System.getProperty("os.name")); put("arch", System.getProperty("os.arch"))
        put("warmup_ms", 1000); put("idle_ms", 3000); put("hinted_samples_per_role", 32); put("missed_samples_per_role", 2)
        put("trials", JsonArray(results))
    }
    val output = Path.of(arguments.single())
    Files.createDirectories(output.parent)
    Files.write(output, (Json { prettyPrint = true }.encodeToString(JsonObject.serializer(), report) + "\n").toByteArray(Charsets.UTF_8))
}
