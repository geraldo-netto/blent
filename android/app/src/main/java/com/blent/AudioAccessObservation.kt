package com.blent

import com.blent.connection.ConnectionAudioAccess
import kotlinx.coroutines.CoroutineScope

/** Application composition for one direction. Authority and actual native cleanup
 * remain in their original adapters; source withdrawal clears only this binding. */
internal class AudioAccessObservation(
    binding: AudioBinding,
    access: ConnectionAudioAccess,
    prompts: LocalPermissionPrompts,
    scope: CoroutineScope,
) {
    val policy = MediaConsentPolicy<AudioAccessOffer>(binding::hasPermission, prompts)
    private val consent = MediaConsentObservation(access.offers, policy, scope)
    private val consumer = MediaConsumerObservation(access.captures, policy.state, binding::hasPermission,
        binding::useTarget, scope, ::AudioConsumerTarget)
    fun retire() { consumer.retire(); consent.retire() }
}
