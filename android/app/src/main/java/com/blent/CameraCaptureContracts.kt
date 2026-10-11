package com.blent

/** Validated local capture metadata; wire/backend types stay in their adapters. */
internal class CameraCaptureTarget(override val endpoint: CameraEndpoint, override val authority: MediaConsumerAuthority) : MediaConsumerTarget<CameraEndpoint> {
    init { require(endpoint.valid()) { "Invalid camera capture target" } }
}

/** Opaque consent lifetime and its original capture authority, projected by a
 * transport adapter. Equal profile values never substitute for this owner. */
internal class CameraCapturePublication(val owner: Any, val target: CameraCaptureTarget)

