package com.blent.connection

import com.blent.*
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.map
import kotlinx.coroutines.flow.combine

/** Wire/presentation adapter. Portable consent sees no credential or worker type.
 * The opaque owner spans one original peer/lease, including permission/admission
 * publications. Matching ticket values in replacement processes never alias it. */
internal class ConnectionCameraAccess(private val binding: ConnectionBinding) {
    private class Original(val peer: ConnectionChoice, val lease: ConnectionResourceLease) { val identity = Any() }
    private var original: Original? = null
    private var published: ConnectionCameraChoice? = null
    private var access: CameraAccessOffer? = null
    // Sample latest state when a conflated wake runs; a queued old snapshot must
    // not clear consent already carried into a newer publication of this owner.
    val offers: Flow<CameraAccessOffer?> = binding.state.map { project(binding.state.value) }
    val captures: Flow<CameraCapturePublication?> = combine(binding.state, binding.cameraTarget) { _, _ -> capture() }

    fun capture(): CameraCapturePublication? {
        val offer = project() ?: return null
        if (!offer.admitted) return null
        val choice = binding.state.value.camera ?: return null
        val target = binding.cameraTarget.value ?: return null
        if (target.endpoint != ConnectionCameraTarget.endpoint(choice.route)) return null
        return CameraCapturePublication(offer.owner, target)
    }

    fun project(view: ConnectionView = binding.state.value): CameraAccessOffer? {
        val choice = currentChoice(view)
        if (choice == null) { clear(); return null }
        val peer = checkNotNull(view.peer)
        if (published === choice && original?.peer === peer) return access
        val owner = original(peer, choice.lease)
        published = choice
        return offer(owner, choice).also { access = it }
    }

    private fun clear() { original = null; published = null; access = null }

    private fun original(peer: ConnectionChoice, lease: ConnectionResourceLease): Original {
        val previous = original
        if (previous?.peer === peer && previous.lease == lease) return previous
        return Original(peer, lease).also { original = it }
    }

    private fun currentChoice(view: ConnectionView): ConnectionCameraChoice? {
        if (!view.active || !view.authenticated || view.peer == null) return null
        val choice = view.camera ?: view.cameraOffer
        if (choice?.lease?.connection != view.peer.ticket) return null
        return choice
    }

    private fun current(owner: Original, choice: ConnectionCameraChoice): Boolean =
        binding.state.value.peer === owner.peer && currentChoice(binding.state.value) === choice

    private fun offer(owner: Original, choice: ConnectionCameraChoice): CameraAccessOffer {
        val profile = profile(choice)
        return CameraAccessOffer(owner.identity, CameraLens.entries.first { it.wire == profile.lens }, profile.background,
            (choice as? ConnectionCameraOfferChoice)?.offer?.permissionRequired == true,
            choice is ConnectionCameraRouteChoice, authority(owner, choice))
    }

    private fun profile(choice: ConnectionCameraChoice): ConnectionManagement.Camera = when (choice) {
        is ConnectionCameraOfferChoice -> choice.offer.profile
        is ConnectionCameraRouteChoice -> choice.route.profile
    }

    private fun authority(owner: Original, choice: ConnectionCameraChoice) = object : CameraAccessAuthority {
        override fun current() = this@ConnectionCameraAccess.current(owner, choice)
        override fun update(context: CameraLocalConsent) = current() && binding.cameraChoice(choice, context.wire())
        override fun permissionResult(granted: Boolean, context: CameraLocalConsent) =
            choice is ConnectionCameraOfferChoice && current() && binding.cameraPermission(choice, granted, context.wire())
    }

    private fun CameraLocalConsent.wire() = CameraConsentChoice(foreground, permission, enabled, backgroundAllowed)
}
