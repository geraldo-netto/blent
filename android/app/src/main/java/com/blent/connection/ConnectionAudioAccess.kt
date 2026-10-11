package com.blent.connection

import com.blent.*

/** Independent audio metadata and original permission boundary. Speaker offers
 * never enter microphone SDK policy. Native capability is composed separately. */
internal class ConnectionAudioAccess(private val binding: ConnectionBinding, private val resource: ConnectionManagement.Resource) {
    init { require(resource == ConnectionManagement.Resource.MICROPHONE || resource == ConnectionManagement.Resource.SPEAKERS) }
    private val access = ConnectionMediaAccess<ConnectionAudioChoice, AudioAccessOffer, AudioEndpoint, AudioConsumerTarget>(
        binding, ::choice, if (resource == ConnectionManagement.Resource.MICROPHONE) binding.microphoneTarget else binding.speakersTarget,
        { (it as? ConnectionAudioRouteChoice)?.route?.let(ConnectionAudioTarget::endpoint) },
        ::offer, binding::audioChoice, ::permission,
    )
    val offers = access.offers
    val captures = access.captures
    fun capture() = access.capture()
    fun project(view: ConnectionView = binding.state.value) = access.project(view)
    private fun choice(view: ConnectionView): ConnectionAudioChoice? =
        if (resource == ConnectionManagement.Resource.MICROPHONE) view.microphone ?: view.microphoneOffer
        else view.speakers ?: view.speakersOffer
    private fun offer(owner: Any, choice: ConnectionAudioChoice, authority: MediaAccessAuthority): AudioAccessOffer {
        val profile = when (choice) {
            is ConnectionAudioOfferChoice -> choice.offer.profile
            is ConnectionAudioRouteChoice -> choice.route.profile
        }
        return AudioAccessOffer(owner, if (resource == ConnectionManagement.Resource.MICROPHONE) 1 else 2,
            profile.background, (choice as? ConnectionAudioOfferChoice)?.offer?.permissionRequired == true,
            choice is ConnectionAudioRouteChoice, authority)
    }
    private fun permission(choice: ConnectionAudioChoice, granted: Boolean, context: MediaLocalConsent) =
        choice is ConnectionAudioOfferChoice && binding.audioPermission(choice, granted, context)
}
