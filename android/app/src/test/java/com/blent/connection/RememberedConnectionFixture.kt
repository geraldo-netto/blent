package com.blent.connection

import kotlinx.coroutines.*

/** Owned virtual-time fixture: logical execution and native retirement are separate. */
internal class RememberedConnectionFixture : AutoCloseable {
    class Worker(val report: (ConnectionWorkerEvent) -> Unit) : ConnectionWorker {
        val ended = CompletableDeferred<Unit>()
        val retired = CompletableDeferred<Unit>()
        val commands = mutableListOf<ConnectionWorkerCommand>()
        var cancelled = 0
        var retirementFailure = false
        override suspend fun execute() { ended.await() }
        override fun cancel() { cancelled++; ended.complete(Unit) }
        override suspend fun retire() { retired.await(); check(!retirementFailure) { "T1393 unconfirmed cleanup" } }
        override fun command(value: ConnectionWorkerCommand): Boolean { commands.add(value); return true }
        fun finish() { ended.complete(Unit); retired.complete(Unit) }
    }
    val dispatcher = ConnectionActorScheduler()
    val scope = CoroutineScope(SupervisorJob() + dispatcher)
    private val bindingScope = CoroutineScope(SupervisorJob() + Dispatchers.Unconfined)
    val workers = mutableListOf<Worker>()
    val binding = ConnectionBinding(bindingScope, { _, report -> Worker(report).also(workers::add) })
    val storage = PreferenceMemoryStorage()
    val preferences = ConnectionPreferenceStore(storage, Dispatchers.Unconfined)
    val configured = ConfiguredConnection(binding, preferences, scope)
    val peer = ConnectionPeer.parse("ab".repeat(32))
    val tablet = ConnectionPeer.parse("cd".repeat(32))
    val ticket = ConnectionTicket(1, peer)
    init { configured.observeRemembered(); configured.observeRemembered(); dispatcher.runCurrent() }
    fun connect(): Worker { configured.connect(); dispatcher.runCurrent(); return workers.last() }
    fun authenticate(worker: Worker, remembered: Boolean = true) {
        worker.report(ConnectionWorkerEvent.Peer(ticket, tablet, remembered))
        worker.report(ConnectionWorkerEvent.State(ConnectionWorkerPhase.AUTHENTICATED))
    }
    fun ready(): Worker = connect().also { authenticate(it); dispatcher.runCurrent() }
    fun end(worker: Worker) { worker.finish(); dispatcher.runCurrent() }
    override fun close() {
        configured.disconnect()
        workers.forEach(Worker::finish)
        scope.cancel(); dispatcher.runCurrent()
        bindingScope.cancel()
        runBlocking { bindingScope.coroutineContext[Job]!!.join(); scope.coroutineContext[Job]!!.join() }
        check(dispatcher.empty()) { "T1393 retained virtual callback or timer" }
    }
}
