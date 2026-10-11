package com.blent

/** Original admitted audio metadata. Construction validates the complete endpoint
 * before a consumer can retain authority or enter a native factory. */
internal class AudioConsumerTarget(
    override val endpoint: AudioEndpoint,
    override val authority: MediaConsumerAuthority,
) : MediaConsumerTarget<AudioEndpoint> {
    init { require(endpoint.valid()) { "Invalid audio consumer target" } }
}

/** Opaque original consent lifetime; equal endpoints are not owner identity. */
internal class AudioConsumerPublication(val owner: Any, val target: AudioConsumerTarget)
