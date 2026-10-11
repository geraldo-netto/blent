package com.blent

/** Serialized on the resource owner's UI dispatcher. Exactly one attached UI
 * launcher and pending action; no OS types or native acquisition. The SDK adapter
 * must reject a second launch while its original result is still outstanding,
 * even when this owner has logically cancelled the action. */
internal class LocalPermissionPrompts {
    private class Attached(val identity: LocalPermissionAttachment, val launch: (LocalPermissionPrompt) -> Boolean)
    private var attached: Attached? = null
    private var pending: LocalPermissionPrompt? = null

    fun attach(launch: (LocalPermissionPrompt) -> Boolean): LocalPermissionAttachment {
        val identity = LocalPermissionAttachment()
        pending = null
        attached = Attached(identity, launch)
        return identity
    }

    fun detach(identity: LocalPermissionAttachment): Boolean {
        if (attached?.identity !== identity) return false
        attached = null
        pending = null
        return true
    }

    fun request(action: LocalPermissionAction): Boolean {
        val owner = attached ?: return false
        if (pending != null || !current(action)) return false
        val prompt = LocalPermissionPrompt(owner.identity, action)
        pending = prompt // Synchronous callbacks may consume this before launch returns.
        val accepted = try { owner.launch(prompt) } catch (_: Exception) { false }
        if (!accepted) cancel(action)
        return accepted
    }

    fun result(prompt: LocalPermissionPrompt, granted: Boolean): Boolean {
        if (pending !== prompt) return false
        pending = null
        if (attached?.identity !== prompt.attachment || !current(prompt.action)) return false
        return try { prompt.action.complete(granted); true } catch (_: Exception) { false }
    }

    private fun current(action: LocalPermissionAction): Boolean = try { action.current() }
        catch (_: Exception) { false }

    fun owns(action: LocalPermissionAction): Boolean = pending?.action === action

    fun cancel(action: LocalPermissionAction) {
        if (pending?.action === action) pending = null
    }

    fun clear() { attached = null; pending = null }
}
