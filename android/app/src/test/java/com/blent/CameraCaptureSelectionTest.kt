package com.blent

import kotlinx.coroutines.flow.MutableStateFlow
import org.junit.Assert.*
import org.junit.Test

class CameraCaptureSelectionTest {
    private val endpoint = CameraEndpoint("ab".repeat(32), 12345, 640, 480, 30, 3000)
    private class Authority : MediaConsumerAuthority {
        var available = true
        var current = true
        var stops = 0
        val proofs = mutableListOf<ResourceRetirement>()
        override fun retain(): MediaConsumerClaim? = if (!available) null else object : MediaConsumerClaim {
            override fun current() = current
            override fun retired(proof: ResourceRetirement) { proofs.add(proof) }
        }
        override fun stop(): Boolean { stops++; current = false; return true }
    }
    private fun offer(owner: Any, background: Boolean = true, admitted: Boolean = true,
                      live: () -> Boolean = { true }) = CameraAccessOffer(owner, CameraLens.REAR,
        background, false, admitted, object : MediaAccessAuthority {
            override fun current() = live()
            override fun update(context: MediaLocalConsent) = true
            override fun permissionResult(granted: Boolean, context: MediaLocalConsent) = false
        })

    @Test fun t932WithdrawnSourceCannotReviveOldWrapperByPublishingEqualMetadata() {
        val owner = Any()
        val authority = Authority()
        val publication = CameraCapturePublication(owner, CameraCaptureTarget(endpoint, authority))
        val view = MediaConsentView<CameraAccessOffer>(offer(owner), true, true, false)
        val selection = CameraCaptureSelection({ view }, { true })
        val first = selection.project(publication)!!
        val claim = first.authority.retain()!!
        assertSame(first, selection.project(CameraCapturePublication(owner, publication.target)))
        selection.project(null)
        assertFalse("T932 source withdrawal must revoke original acquisition authority", claim.current())
        assertNull(first.authority.retain())
        assertNotSame(first, selection.project(publication))
        assertFalse("T932 equal metadata cannot revive the original wrapper", claim.current())
        claim.retired(ResourceRetirement.RETIRED)
        assertEquals(listOf(ResourceRetirement.RETIRED), authority.proofs)
    }

    @Test fun t932EveryConsentAndPermissionBooleanRejectsUnsafeCapture() {
        for (bits in 0 until 128) {
            val enabled = bits and 1 != 0
            val admitted = bits and 2 != 0
            val permission = bits and 4 != 0
            val live = bits and 8 != 0
            val foreground = bits and 16 != 0
            val background = bits and 32 != 0
            val requested = bits and 64 != 0
            val owner = Any()
            val view = MediaConsentView<CameraAccessOffer>(offer(owner, requested, admitted) { live }, enabled, foreground, background)
            val selection = CameraCaptureSelection({ view }, { permission })
            val value = CameraCapturePublication(owner, CameraCaptureTarget(endpoint, Authority()))
            val allowed = enabled && admitted && permission && live && (foreground || (background && requested))
            assertEquals("T932 bits=$bits", allowed, selection.project(value) != null)
        }
    }

    @Test fun t932FreshStateOriginalOwnerAndClaimAreCheckedAtAcquisition() {
        val owner = Any()
        val authority = Authority()
        val view = MutableStateFlow(MediaConsentView<CameraAccessOffer>(offer(owner), true, true, false))
        var permission = true
        val selection = CameraCaptureSelection({ view.value }, { permission })
        val target = selection.project(CameraCapturePublication(owner, CameraCaptureTarget(endpoint, authority)))!!
        val claim = target.authority.retain()!!
        permission = false
        assertFalse(claim.current()); assertNull(target.authority.retain())
        permission = true
        view.value = view.value.copy(enabled = false)
        assertFalse(claim.current()); assertNull(target.authority.retain())
        view.value = view.value.copy(enabled = true, offer = offer(Any()))
        assertFalse(claim.current()); assertNull(target.authority.retain())
        view.value = view.value.copy(offer = offer(owner))
        authority.available = false
        assertNull(target.authority.retain())
        authority.current = false
        assertFalse(claim.current())
        assertTrue(target.authority.stop())
        assertEquals(1, authority.stops)
        claim.retired(ResourceRetirement.NOT_STARTED)
        assertEquals(listOf(ResourceRetirement.NOT_STARTED), authority.proofs)
    }

    @Test fun t932MissingConsentAndThrowingPermissionOrAuthorityFailClosed() {
        val owner = Any()
        val value = CameraCapturePublication(owner, CameraCaptureTarget(endpoint, Authority()))
        assertNull(CameraCaptureSelection({ MediaConsentView<CameraAccessOffer>() }, { true }).project(value))
        val valid = MediaConsentView<CameraAccessOffer>(offer(owner), true, true, false)
        assertNull(CameraCaptureSelection({ valid }, { throw SecurityException() }).project(value))
        assertNull(CameraCaptureSelection({ valid.copy(offer = offer(owner) { throw IllegalStateException() }) },
            { true }).project(value))
    }
}
