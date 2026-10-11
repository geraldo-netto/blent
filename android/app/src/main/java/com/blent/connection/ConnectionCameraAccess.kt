package com.blent.connection

import com.blent.*

/** Camera metadata and command boundary around shared original publication ownership. */
internal class ConnectionCameraAccess(private val binding: ConnectionBinding) {
    private val access = ConnectionMediaAccess<ConnectionCameraChoice, CameraAccessOffer, CameraEndpoint, CameraCaptureTarget>(
        binding, { it.camera ?: it.cameraOffer }, binding.cameraTarget,
        { (it as? ConnectionCameraRouteChoice)?.route?.let(ConnectionCameraTarget::endpoint) },
        ::offer, { choice, context -> binding.cameraChoice(choice, context.wire()) }, ::permission,
    )
    val offers = access.offers
    val captures = access.captures
    fun capture() = access.capture()
    fun project(view: ConnectionView = binding.state.value) = access.project(view)
    private fun offer(owner: Any, choice: ConnectionCameraChoice, authority: MediaAccessAuthority): CameraAccessOffer {
        val profile = profile(choice)
        return CameraAccessOffer(owner, CameraLens.entries.first { it.wire == profile.lens }, profile.background,
            (choice as? ConnectionCameraOfferChoice)?.offer?.permissionRequired == true,
            choice is ConnectionCameraRouteChoice, authority)
    }
    private fun profile(choice: ConnectionCameraChoice): ConnectionManagement.Camera = when (choice) {
        is ConnectionCameraOfferChoice -> choice.offer.profile
        is ConnectionCameraRouteChoice -> choice.route.profile
    }
    private fun permission(choice: ConnectionCameraChoice, granted: Boolean, context: MediaLocalConsent) =
        choice is ConnectionCameraOfferChoice && binding.cameraPermission(choice, granted, context.wire())
    private fun MediaLocalConsent.wire() = CameraConsentChoice(foreground, permission, enabled, backgroundAllowed)
}
