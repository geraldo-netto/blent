package com.blent.connection

import kotlinx.coroutines.*
import org.junit.Assert.*
import org.junit.Test

private enum class PreferenceAction { OWNER_LOST, READ, SAVE, ORIGINAL_RETURN }
private fun preferenceOrders(prefix: List<PreferenceAction>, remaining: List<PreferenceAction>): List<List<PreferenceAction>> {
    if (remaining.isEmpty()) return listOf(prefix)
    return remaining.flatMap { next -> preferenceOrders(prefix + next, remaining - next) }
}

/** Independent four-event oracle: owner loss cannot free an original API slot.
 * Probes after actual return are settled separately before the next event. */
private class PreferenceHistory(write: Boolean) : AutoCloseable {
    private val dispatcher = ConnectionActorScheduler()
    private val storage = PreferenceMemoryStorage()
    private val store = ConnectionPreferenceStore(storage, dispatcher)
    private val owner = CoroutineScope(SupervisorJob() + Dispatchers.Unconfined)
    private val original = owner.async(start = CoroutineStart.UNDISPATCHED) {
        if (write) store.save(ConnectionPreferences.fields(ConnectionWorkerSpec("usb"))) else store.load()
    }
    private var pending = true

    suspend fun event(action: PreferenceAction) {
        when (action) {
            PreferenceAction.OWNER_LOST -> loseOwner()
            PreferenceAction.READ -> probe(false)
            PreferenceAction.SAVE -> probe(true)
            PreferenceAction.ORIGINAL_RETURN -> complete()
        }
    }
    private fun loseOwner() {
        owner.cancel()
        if (pending) assertFalse(original.isCompleted)
    }
    private suspend fun complete() {
        dispatcher.runCurrent(); original.join()
        pending = false
    }
    private suspend fun probe(save: Boolean) = coroutineScope {
        val effects = storage.reads + storage.writes
        val attempt = async(start = CoroutineStart.UNDISPATCHED) {
            if (save) store.save(ConnectionPreferences.fields(ConnectionWorkerSpec("usb"))) else store.load()
        }
        try {
            if (pending) rejected(attempt, effects)
            else { dispatcher.runCurrent(); assertTrue(attempt.await().isSuccess) }
        } finally {
            attempt.cancel()
            if (!attempt.isCompleted) dispatcher.runCurrent()
            attempt.join()
        }
    }
    private suspend fun rejected(attempt: Deferred<Result<ConnectionWorkerSpec>>, effects: Int) {
        assertTrue("A competing API call must reject before queueing native work", attempt.isCompleted)
        assertTrue(attempt.await().exceptionOrNull()?.message.orEmpty().contains("busy"))
        assertEquals(effects, storage.reads + storage.writes)
    }
    override fun close() {
        owner.cancel()
        dispatcher.runCurrent()
        runBlocking { original.join() }
        assertTrue(dispatcher.empty())
    }
}

class ConnectionPreferenceHistoryTest {
    @Test fun t1099AllBoundedReadWriteOwnerLossAdmissionAndReturnOrdersPreserveOriginalSlot() = runBlocking {
        val histories = preferenceOrders(emptyList(), PreferenceAction.entries)
        assertEquals(24, histories.size)
        assertEquals(24, histories.toSet().size)
        for (write in listOf(false, true)) for (history in histories) PreferenceHistory(write).use { run ->
            for (action in history) run.event(action)
        }
    }
}
