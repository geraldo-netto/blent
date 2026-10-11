package com.blent

/** Original admitted audio metadata. Construction validates the complete endpoint
 * before a consumer can retain authority or enter a native factory. */
internal class AudioConsumerTarget(
    override val endpoint: AudioEndpoint,
    override val authority: MediaConsumerAuthority,
) : MediaConsumerTarget<AudioEndpoint> {
    init { require(endpoint.valid()) { "Invalid audio consumer target" } }
}
