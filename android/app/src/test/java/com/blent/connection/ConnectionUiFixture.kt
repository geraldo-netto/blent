package com.blent.connection

import kotlinx.coroutines.*
import org.robolectric.util.ReflectionHelpers

/** Scoped replacement of the process owner, restoring it only after test workers
 * retire. Never adds a production reset that could forget a quarantined owner. */
internal class ConnectionUiFixture : AutoCloseable {
    class Worker(val report: (ConnectionWorkerEvent) -> Unit) : ConnectionWorker {
        val released = CompletableDeferred<Unit>()
        val commands = java.util.concurrent.CopyOnWriteArrayList<ConnectionWorkerCommand>()
        var cancelled = 0
        override suspend fun execute() { released.await() }
        override fun cancel() { cancelled++ }
        override suspend fun retire() { released.await() }
        override fun command(value: ConnectionWorkerCommand): Boolean { commands.add(value); return true }
    }
    val scope = CoroutineScope(SupervisorJob() + Dispatchers.Unconfined)
    val preferences = ConnectionPreferenceStore(PreferenceMemoryStorage(), Dispatchers.Unconfined)
    val workers = mutableListOf<Worker>()
    val specifications = mutableListOf<ConnectionWorkerSpec>()
    private val access = ReflectionHelpers.getStaticField<ConnectionAccess>(ConnectionOwner::class.java, "access")
    val binding = ConnectionBinding(scope, { spec, report -> specifications.add(spec); Worker(report).also { workers.add(it) } }, access = access)
    val savedRequests = mutableListOf<SavedTrustRequest>()
    val savedRuns = mutableListOf<SavedTrustUiRun>()
    val saved = SavedTrustBinding(scope, access, SavedTrustRunFactory { request, report ->
        savedRequests.add(request); SavedTrustUiRun(report).also { savedRuns.add(it) }
    })
    val computer = ConnectionPeer.parse("ab".repeat(32))
    val tablet = ConnectionPeer.parse("cd".repeat(32))
    val ticket = ConnectionTicket(1, computer)
    private val previous = ReflectionHelpers.getStaticField<ConnectionBinding?>(ConnectionOwner::class.java, "binding")
    private val previousSaved = ReflectionHelpers.getStaticField<SavedTrustBinding?>(ConnectionOwner::class.java, "saved")
    private val previousPreferences = ReflectionHelpers.getStaticField<ConnectionPreferenceStore?>(AndroidConnectionPreferences::class.java, "store")
    init {
        ReflectionHelpers.setStaticField(ConnectionOwner::class.java, "binding", binding)
        ReflectionHelpers.setStaticField(ConnectionOwner::class.java, "saved", saved)
        ReflectionHelpers.setStaticField(AndroidConnectionPreferences::class.java, "store", preferences)
    }
    fun prompt() { workers.last().report(ConnectionWorkerEvent.Pairing(ticket, tablet)) }
    fun connect() { binding.connect(ConnectionWorkerSpec("usb")) }
    override fun close() {
        binding.disconnect()
        workers.forEach { it.released.complete(Unit) }
        savedRuns.forEach { it.release.complete(Unit); it.finish.complete(Unit) }
        scope.cancel()
        kotlinx.coroutines.runBlocking { scope.coroutineContext[Job]!!.join() }
        ReflectionHelpers.setStaticField(ConnectionOwner::class.java, "binding", previous)
        ReflectionHelpers.setStaticField(ConnectionOwner::class.java, "saved", previousSaved)
        ReflectionHelpers.setStaticField(AndroidConnectionPreferences::class.java, "store", previousPreferences)
    }
}
