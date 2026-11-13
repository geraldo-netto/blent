package com.blent.connection

import kotlinx.coroutines.*
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.first

internal data class ConfiguredConnectionView(val pending: Boolean = false, val failure: String? = null)
internal fun connectionPreferenceFailure(error: Throwable): String = (error.message ?: "USB settings request failed.").take(256)

/** One Activity's bounded start request. Original job completion fences its slot;
 * the application store separately retains any original native storage call. */
internal class ConfiguredConnection(
    private val binding: ConnectionBinding,
    val preferences: ConnectionPreferenceStore,
    private val scope: CoroutineScope,
) {
    private enum class Request { MANUAL, ATTACHMENT, RETRY }
    private data class Selection(val attempt: Any?, val active: Boolean)
    private class Attempt(val identity: Any, val execution: Job)
    private var current: Attempt? = null
    private var remembered: Job? = null
    private val mutableState = MutableStateFlow(ConfiguredConnectionView())
    val state = mutableState.asStateFlow()

    @Synchronized fun connect() {
        if (current != null || binding.state.value.active) return
        start(Request.MANUAL)
    }

    /** Coalesce attachment notifications without canceling a healthy worker.
     * Explicit user cutoff cancels a waiting restart; ResourceLane fences the
     * replacement behind original native retirement. */
    @Synchronized fun attach() {
        if (current != null) return
        start(Request.ATTACHMENT)
    }

    @Synchronized fun observeRemembered() {
        if (remembered != null) return
        remembered = scope.launch { RememberedConnection(binding.state, ::retry).run() }
    }

    private suspend fun retry(attempt: Any): Any? = admitRetry(attempt)?.await()

    @Synchronized private fun admitRetry(attempt: Any): Deferred<Any?>? {
        val currentView = binding.state.value
        if (current != null || currentView.active) return null
        if (currentView.attempt !== attempt || !currentView.reconnectAllowed) return null
        return start(Request.RETRY)
    }

    private fun start(request: Request): Deferred<Any?> {
        val identity = Any()
        val initial = binding.state.value.let { Selection(it.attempt, it.active) }
        val outcome = CompletableDeferred<Any?>()
        val execution = scope.launch(start = CoroutineStart.LAZY) { outcome.complete(run(identity, request, initial)) }
        current = Attempt(identity, execution)
        mutableState.value = ConfiguredConnectionView(pending = true)
        execution.invokeOnCompletion { outcome.complete(null); finished(identity) }
        execution.start()
        return outcome
    }

    private suspend fun run(identity: Any, request: Request, initial: Selection): Any? {
        if (request != Request.MANUAL && !attachmentReady(initial)) return null
        val selected = preferences.load()
        currentCoroutineContext().ensureActive()
        return admit(identity, request, initial, selected)
    }

    @Synchronized private fun admit(identity: Any, request: Request, initial: Selection,
        selected: Result<ConnectionWorkerSpec>): Any? {
        if (current?.identity !== identity) return null
        if (request != Request.MANUAL && binding.state.value.attempt !== initial.attempt) return null
        if (request == Request.RETRY && !binding.state.value.reconnectAllowed) return null
        val failure = selected.exceptionOrNull()
        if (failure != null) {
            mutableState.value = ConfiguredConnectionView(true, connectionPreferenceFailure(failure))
            return null
        }
        if (binding.state.value.active) return null
        binding.connect(selected.getOrThrow())
        return binding.state.value.attempt.takeIf { it !== initial.attempt }
    }

    private suspend fun attachmentReady(initial: Selection): Boolean {
        if (!initial.active) return true
        val ended = binding.state.first { !it.active || it.attempt !== initial.attempt }
        return ended.attempt === initial.attempt && ended.reconnectAllowed
    }

    @Synchronized private fun finished(identity: Any) {
        if (current?.identity !== identity) return
        current = null
        mutableState.value = mutableState.value.copy(pending = false)
    }

    @Synchronized fun disconnect() {
        current?.execution?.cancel()
        binding.disconnect()
    }
}
