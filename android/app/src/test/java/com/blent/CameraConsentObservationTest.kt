package com.blent

import kotlinx.coroutines.*
import kotlinx.coroutines.flow.*
import org.junit.Assert.*
import org.junit.Test

class CameraConsentObservationTest {
    private class Authority : MediaAccessAuthority {
        val updates = mutableListOf<MediaLocalConsent>()
        override fun current() = true
        override fun update(context: MediaLocalConsent) = updates.add(context)
        override fun permissionResult(granted: Boolean, context: MediaLocalConsent) = false
    }
    private class Rig : AutoCloseable {
        val scope = CoroutineScope(SupervisorJob() + Dispatchers.Unconfined)
        val policy = MediaConsentPolicy<CameraAccessOffer>({ true }, LocalPermissionPrompts())
        val source = MutableStateFlow<CameraAccessOffer?>(null)
        val observer = MediaConsentObservation<CameraAccessOffer>(source, policy, scope)
        val authorities = mutableListOf<Authority>()
        fun offer(): CameraAccessOffer {
            val authority = Authority().also { authorities.add(it) }
            return CameraAccessOffer(Any(), CameraLens.FRONT, true, false, false, authority)
        }
        override fun close() { observer.retire(); scope.cancel() }
    }

    @Test fun t944ProcessObservationRetainsExplicitBackgroundIntentAndDropsOldSources() = Rig().use { rig ->
        val first = rig.offer()
        rig.source.value = first
        rig.policy.foreground(true)
        assertTrue(rig.policy.start(first)); assertTrue(rig.policy.background(first, true))
        rig.policy.foreground(false)
        assertTrue(rig.policy.state.value.enabled)
        val next = rig.offer()
        rig.source.value = next
        assertSame(next, rig.policy.state.value.offer)
        assertFalse(rig.policy.state.value.enabled)
        assertFalse(rig.authorities.first().updates.last().enabled)
        assertFalse(rig.policy.start(first))
        rig.observer.retire(); rig.observer.retire()
        rig.source.value = rig.offer()
        assertNull(rig.policy.state.value.offer)
        assertFalse(rig.policy.start(next))
    }

    @Test fun t944NormalSourceEndAndSourceFailureRevokeOnlyThisPolicy() = runBlocking {
        for (failed in listOf(false, true)) {
            val policy = MediaConsentPolicy<CameraAccessOffer>({ true }, LocalPermissionPrompts())
            val source = MutableSharedFlow<CameraAccessOffer?>()
            val scope = CoroutineScope(SupervisorJob() + Dispatchers.Unconfined)
            val finished = CompletableDeferred<Unit>()
            val sibling = scope.launch { finished.await() }
            val authority = Authority()
            val offer = CameraAccessOffer(Any(), CameraLens.REAR, false, false, false, authority)
            val stream = flow {
                emit(offer)
                policy.foreground(true); policy.start(offer)
                source.first()
                if (failed) error("camera source failed")
            }
            val observer = MediaConsentObservation<CameraAccessOffer>(stream, policy, scope)
            assertSame(offer, policy.state.value.offer)
            source.emit(null)
            assertNull(policy.state.value.offer)
            assertFalse(authority.updates.last().enabled)
            assertTrue(sibling.isActive)
            observer.retire(); finished.complete(Unit); sibling.join(); scope.cancel()
        }
    }

    @Test fun t944CancellationAndQueuedLatePublicationCannotRestoreAccess() = Rig().use { rig ->
        val offered = rig.offer()
        rig.source.value = offered
        rig.scope.cancel()
        assertNull(rig.policy.state.value.offer)
        rig.source.value = rig.offer()
        assertNull(rig.policy.state.value.offer)
        rig.observer.retire()
    }
}
