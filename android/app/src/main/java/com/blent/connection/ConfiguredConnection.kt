package com.blent.connection

import kotlinx.coroutines.*
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.asStateFlow

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
        val identity = Any()
        val execution = scope.launch(start = CoroutineStart.LAZY) { run(identity) }
        current = Attempt(identity, execution)
        mutableState.value = ConfiguredConnectionView(pending = true)
        execution.invokeOnCompletion { finished(identity) }
        execution.start()
    }

    private suspend fun run(identity: Any) {
        val selected = preferences.load()
        currentCoroutineContext().ensureActive()
        synchronized(this) {
            if (current?.identity !== identity) return
            val failure = selected.exceptionOrNull()
            if (failure != null) mutableState.value = ConfiguredConnectionView(true, connectionPreferenceFailure(failure))
            else if (!binding.state.value.active) binding.connect(selected.getOrThrow())
        }
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
