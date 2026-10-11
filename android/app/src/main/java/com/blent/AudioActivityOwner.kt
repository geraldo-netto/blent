package com.blent

/** Application-owned foreground adapter. One original attachment controls both
 * independent audio bindings; stopped Activities cannot detach replacements.
 * Permission actions contain resource identity, never an Activity reference. */
internal class AudioActivityOwner(
    private val microphone: AudioBinding,
    private val speakers: AudioBinding,
    private val prompts: LocalPermissionPrompts,
) {
    private var foreground = false
    private var closed = false
    private var pending: LocalPermissionAction? = null

    fun attach(launch: (LocalPermissionPrompt) -> Boolean): LocalPermissionAttachment {
        check(!closed) { "Audio UI owner retired" }
        val identity = prompts.attach(launch)
        pending = null
        foreground = true
        microphone.start()
        speakers.start()
        if (microphone.permissionPending != null) requestPermission()
        return identity
    }

    fun detach(identity: LocalPermissionAttachment): Boolean {
        if (!prompts.detach(identity)) return false
        pending = null
        foreground = false
        microphone.stop()
        speakers.stop()
        return true
    }

    fun requestPermission(): Boolean {
        val original = microphone.permissionPending ?: return false
        if (pending?.let { prompts.owns(it) && it.current() } == true) return true
        val action = action(original)
        pending = action
        if (!closed && foreground && prompts.request(action)) return true
        if (pending === action) pending = null
        microphone.permissionResult(false, original)
        return false
    }

    private fun action(original: AudioCapturePermission) = object : LocalPermissionAction {
        override fun current() = pending === this && !closed && foreground &&
            microphone.permissionPending === original
        override fun complete(granted: Boolean) {
            if (!current()) return
            pending = null
            microphone.permissionResult(granted, original)
        }
    }

    fun clear() {
        closed = true
        pending = null
        foreground = false
        prompts.clear()
        microphone.shutdown()
        speakers.shutdown()
    }
}
