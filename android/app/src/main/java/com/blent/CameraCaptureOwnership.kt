package com.blent

/** Explicit target takes precedence; retain latest ADB invitation for fallback. */
internal class CameraCaptureSource {
    private var invitation: CameraEndpoint? = null
    var target: CameraCaptureTarget? = null; private set
    val endpoint: CameraEndpoint? get() = target?.endpoint ?: invitation
    fun invitation(value: CameraEndpoint?): Boolean {
        if (invitation == value) return false
        invitation = value
        return target == null
    }
    fun target(value: CameraCaptureTarget?): Boolean {
        if (target === value) return false
        target = value
        return true
    }
}
