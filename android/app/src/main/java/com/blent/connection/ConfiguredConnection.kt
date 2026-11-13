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
    private class Attempt(val identity: Any, val execution: Job)
    private var current: Attempt? = null
    private val mutableState = MutableStateFlow(ConfiguredConnectionView())
    val state = mutableState.asStateFlow()

    @Synchronized fun connect() {
        if (current != null || binding.state.value.active) return
        start(false)
    }

    /** Coalesce attachment notifications without canceling a healthy worker.
     * Explicit user cutoff cancels a waiting restart; ResourceLane fences the
     * replacement behind original native retirement. */
    @Synchronized fun attach() {
        if (current != null) return
        start(true)
    }

    private fun start(attachment: Boolean) {
        val identity = Any()
        val initial = binding.state.value
        val execution = scope.launch(start = CoroutineStart.LAZY) { run(identity, attachment, initial) }
        current = Attempt(identity, execution)
        mutableState.value = ConfiguredConnectionView(pending = true)
        execution.invokeOnCompletion { finished(identity) }
        execution.start()
    }

    private suspend fun run(identity: Any, attachment: Boolean, initial: ConnectionView) {
        if (attachment && !attachmentReady(initial)) return
        val selected = preferences.load()
        currentCoroutineContext().ensureActive()
        synchronized(this) {
            if (current?.identity !== identity) return
            if (attachment && binding.state.value.attempt !== initial.attempt) return
            val failure = selected.exceptionOrNull()
            if (failure != null) mutableState.value = ConfiguredConnectionView(true, connectionPreferenceFailure(failure))
            else if (!binding.state.value.active) binding.connect(selected.getOrThrow())
        }
    }

    private suspend fun attachmentReady(initial: ConnectionView): Boolean {
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
