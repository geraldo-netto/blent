package com.blent

/** Validated local capture metadata; wire/backend types stay in their adapters. */
internal class CameraCaptureTarget(override val endpoint: CameraEndpoint, override val authority: MediaConsumerAuthority) : MediaConsumerTarget<CameraEndpoint> {
    init { require(endpoint.valid()) { "Invalid camera capture target" } }
}
