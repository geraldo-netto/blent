package com.blent

/** One possible native request, retained separately from its route's visibility.
 * Only actual retirement/no-start releases it. Implementations must bind current
 * to the original owner, never a mutable latest connection or matching counter. */
internal interface CameraCaptureClaim {
    fun current(): Boolean
    fun retired(proof: ResourceRetirement)
}

/** Focused portable controls. Retain refuses a closed target; stop requests only
 * this camera's closure and cannot substitute for actual native completion. */
internal interface CameraCaptureAuthority {
    fun retain(): CameraCaptureClaim?
    fun stop(): Boolean
}

/** Validated local capture metadata; wire/backend types stay in their adapters. */
internal class CameraCaptureTarget(val endpoint: CameraEndpoint, val authority: CameraCaptureAuthority) {
    init { require(endpoint.valid()) { "Invalid camera capture target" } }
}

