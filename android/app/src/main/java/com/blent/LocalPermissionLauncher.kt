package com.blent

/** One Activity's actual SDK request slot. Logical prompt cancellation cannot
 * free this slot: keep the original request until its SDK result or destruction.
 * Calls serialize on the UI dispatcher. */
internal class LocalPermissionLauncher(
    private val launchSdk: () -> Unit,
    private val publish: (LocalPermissionPrompt, Boolean) -> Unit,
) {
    private var pending: LocalPermissionPrompt? = null
    private var closed = false

    fun launch(prompt: LocalPermissionPrompt): Boolean {
        if (closed || pending != null) return false
        pending = prompt
        try { launchSdk() }
        catch (failure: Exception) { if (pending === prompt) pending = null; throw failure }
        return true
    }

    fun complete(granted: Boolean): Boolean {
        val original = pending ?: return false
        pending = null
        publish(original, granted)
        return true
    }

    fun retire() { closed = true; pending = null }
}
