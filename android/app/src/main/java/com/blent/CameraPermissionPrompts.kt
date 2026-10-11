package com.blent

/** Serialized on the application's camera dispatcher. Exactly one attached UI
 * launcher and pending action; no OS types or native acquisition. The SDK adapter
 * must reject a second launch while its original result is still outstanding,
 * even when this owner has logically cancelled the action. */
internal class CameraPermissionPrompts {
    private class Attached(val identity: CameraPermissionAttachment, val launch: (CameraPermissionPrompt) -> Boolean)
    private var attached: Attached? = null
    private var pending: CameraPermissionPrompt? = null

    fun attach(launch: (CameraPermissionPrompt) -> Boolean): CameraPermissionAttachment {
        val identity = CameraPermissionAttachment()
        pending = null
        attached = Attached(identity, launch)
        return identity
    }

    fun detach(identity: CameraPermissionAttachment): Boolean {
        if (attached?.identity !== identity) return false
        attached = null
        pending = null
        return true
    }

    fun request(action: CameraPermissionAction): Boolean {
        val owner = attached ?: return false
        if (pending != null || !current(action)) return false
        val prompt = CameraPermissionPrompt(owner.identity, action)
        pending = prompt // Synchronous callbacks may consume this before launch returns.
        val accepted = try { owner.launch(prompt) } catch (_: Exception) { false }
        if (!accepted) cancel(action)
        return accepted
    }

    fun result(prompt: CameraPermissionPrompt, granted: Boolean): Boolean {
        if (pending !== prompt) return false
        pending = null
        if (attached?.identity !== prompt.attachment || !current(prompt.action)) return false
        return try { prompt.action.complete(granted); true } catch (_: Exception) { false }
    }

    private fun current(action: CameraPermissionAction): Boolean = try { action.current() }
        catch (_: Exception) { false }

    fun owns(action: CameraPermissionAction): Boolean = pending?.action === action

    fun cancel(action: CameraPermissionAction) {
        if (pending?.action === action) pending = null
    }

    fun clear() { attached = null; pending = null }
}
