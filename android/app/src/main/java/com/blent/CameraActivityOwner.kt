package com.blent

/** Application-owned camera UI lifecycle. Only the attached broker launcher may
 * capture an Activity; permission actions retain original portable state only.
 * Native release remains CameraBinding's retirement lane, not UI detachment. */
internal class CameraActivityOwner(
    private val binding: CameraBinding,
    val consent: CameraConsentPolicy,
    private val prompts: CameraPermissionPrompts,
) {
    private var foreground = false
    private var closed = false
    private var legacy: CameraPermissionAction? = null

    fun attach(launch: (CameraPermissionPrompt) -> Boolean): CameraPermissionAttachment {
        check(!closed) { "Camera UI owner retired" }
        val identity = prompts.attach(launch)
        foreground = true
        consent.foreground(true)
        binding.start()
        if (binding.permissionPending != null) requestLegacy()
        return identity
    }

    fun detach(identity: CameraPermissionAttachment): Boolean {
        if (!prompts.detach(identity)) return false
        legacy = null
        foreground = false
        consent.foreground(false)
        binding.stop()
        return true
    }

    fun requestLegacy(): Boolean {
        val original = binding.permissionPending ?: return false
        if (legacy?.let { prompts.owns(it) && it.current() } == true) return true
        val action = legacyAction(original)
        legacy = action
        if (!closed && foreground && prompts.request(action)) return true
        if (legacy === action) legacy = null
        binding.permissionResult(false, original)
        return false
    }

    private fun legacyAction(original: CameraCapturePermission) = object : CameraPermissionAction {
        override fun current() = legacy === this && !closed && foreground && binding.permissionPending === original
        override fun complete(granted: Boolean) {
            if (!current()) return
            legacy = null
            binding.permissionResult(granted, original)
        }
    }

    fun clear() {
        closed = true
        legacy = null
        foreground = false
        prompts.clear()
        consent.clear()
        binding.shutdown()
    }
}
