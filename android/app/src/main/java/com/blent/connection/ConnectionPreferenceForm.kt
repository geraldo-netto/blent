package com.blent.connection

import androidx.compose.runtime.*

/** One dialog's editable draft; caller serializes all actions on the UI thread.
 * Eleven values, each at most 20 input characters; no active worker reference. */
internal class ConnectionPreferenceForm {
    var fields by mutableStateOf(ConnectionPreferences.fields(ConnectionWorkerSpec("usb")))
        private set
    var busy by mutableStateOf(false)
        private set
    var error by mutableStateOf<String?>(null)
        private set
    var notice by mutableStateOf<String?>(null)
        private set

    fun edit(field: ConnectionPreferenceField, value: String) {
        if (busy) return
        if (value.length > 20) { error = "Use at most seven decimal digits."; return }
        fields = fields + (field.key to value)
        error = null; notice = null
    }
    fun invalid(field: ConnectionPreferenceField): Boolean = try {
        field.update(ConnectionWorkerSpec("usb"), fields.getValue(field.key)); false
    } catch (_: IllegalArgumentException) { true }
    fun defaults() {
        if (busy) return
        fields = ConnectionPreferences.fields(ConnectionWorkerSpec("usb"))
        error = null; notice = "Defaults selected. Save for the next USB connection."
    }
    suspend fun load(store: ConnectionPreferenceStore) = request(false) { store.load() }
    suspend fun save(store: ConnectionPreferenceStore) = request(true) { store.save(fields) }
    private suspend fun request(saved: Boolean, work: suspend () -> Result<ConnectionWorkerSpec>) {
        if (busy) return
        busy = true; error = null; notice = null
        try { publish(work(), saved) }
        finally { busy = false }
    }
    private fun publish(result: Result<ConnectionWorkerSpec>, saved: Boolean) {
        val failure = result.exceptionOrNull()
        if (failure != null) { error = connectionPreferenceFailure(failure); return }
        fields = ConnectionPreferences.fields(result.getOrThrow())
        if (saved) notice = "Saved for the next USB connection."
    }
}
