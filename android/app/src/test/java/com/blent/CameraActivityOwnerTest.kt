package com.blent

import kotlinx.coroutines.*
import kotlinx.coroutines.flow.MutableStateFlow
import org.junit.Assert.*
import org.junit.Test

class CameraActivityOwnerTest {
    private val endpoint = CameraEndpoint("ab".repeat(32), 12345, 640, 480, 30, 1000, CameraLens.FRONT)
    private inner class Rig : AutoCloseable {
        val scope = CoroutineScope(SupervisorJob() + Dispatchers.Unconfined)
        val invitations = MutableStateFlow<CameraEndpoint?>(null)
        val prompts = LocalPermissionPrompts()
        var allowed = false
        val captures = mutableListOf<CameraLens>()
        val services = mutableListOf<String?>()
        lateinit var owner: CameraActivityOwner
        val binding = CameraBinding({ 0 }, { owner.requestLegacy() }, { allowed }, { _, lens, _, _ ->
            object : LifecycleRun {
                override suspend fun execute() { captures.add(lens); awaitCancellation() }
                override fun cancel() {}
                override suspend fun retire() {}
            }
        }, invitations, scope, services::add)
        init { owner = CameraActivityOwner(binding, MediaConsentPolicy<CameraAccessOffer>({ allowed }, prompts), prompts) }
        fun launcher(native: () -> Unit = {}) = LocalPermissionLauncher(native) { prompt, granted -> prompts.result(prompt, granted) }
        override fun close() { owner.clear(); scope.cancel() }
    }

    @Test fun t961AlreadyAvailableInvitationKeepsOneOriginalPromptOnAttach() = Rig().use { rig ->
        var launches = 0
        val sdk = rig.launcher { launches++ }
        rig.invitations.value = endpoint
        rig.owner.attach(sdk::launch)
        assertNotNull("T961 attach consumed its own newly launched request", rig.binding.permissionPending)
        assertEquals(1, launches)
        rig.allowed = true
        assertTrue(sdk.complete(true))
        assertEquals(listOf(CameraLens.FRONT), rig.captures)
    }

    @Test fun t961OldDetachCannotStopReplacementOrClearItsPrompt() = Rig().use { rig ->
        val old = rig.owner.attach(rig.launcher()::launch)
        val next = rig.launcher()
        val latest = rig.owner.attach(next::launch)
        assertFalse(rig.owner.detach(old))
        assertTrue(rig.owner.consent.state.value.foreground)
        rig.invitations.value = endpoint
        val original = rig.binding.permissionPending!!
        rig.allowed = true
        assertTrue(next.complete(true))
        assertNull(rig.binding.permissionPending)
        assertEquals(listOf(CameraLens.FRONT), rig.captures)
        rig.binding.permissionResult(false, original)
        assertTrue(rig.owner.detach(latest))
        assertNull(rig.binding.endpoint)
        assertFalse(rig.owner.detach(latest))
    }

    @Test fun t961ReplacementLaunchRetainsExactRequestAndRejectsOldSdkResult() = Rig().use { rig ->
        val old = rig.launcher()
        rig.owner.attach(old::launch)
        rig.invitations.value = endpoint
        val request = rig.binding.permissionPending!!
        val next = rig.launcher()
        rig.owner.attach(next::launch)
        rig.allowed = true
        assertTrue(old.complete(true))
        assertSame("T961 stale Activity result consumed current request", request, rig.binding.permissionPending)
        assertTrue(rig.captures.isEmpty())
        assertTrue(next.complete(true))
        assertEquals(listOf(CameraLens.FRONT), rig.captures)
        assertFalse(next.complete(false))
    }

    @Test fun t961BusySdkSlotCannotRelabelOriginalGrantToReplacementLens() = Rig().use { rig ->
        var launches = 0
        val sdk = rig.launcher { launches++ }
        rig.owner.attach(sdk::launch)
        rig.invitations.value = endpoint
        rig.binding.choose(CameraLens.REAR)
        assertEquals(1, launches)
        assertNull(rig.binding.permissionPending)
        rig.allowed = true
        assertTrue(sdk.complete(true))
        assertTrue(rig.captures.isEmpty())
        rig.binding.choose(CameraLens.REAR)
        assertEquals(listOf(CameraLens.REAR), rig.captures)
    }

    @Test fun t961DetachKeepsOnlyExplicitBackgroundCaptureAndClearRetiresOwner() = Rig().use { rig ->
        rig.allowed = true
        val attachment = rig.owner.attach(rig.launcher()::launch)
        rig.invitations.value = endpoint.copy(background = true)
        assertEquals(CameraLens.FRONT, rig.binding.selected)
        assertTrue(rig.owner.detach(attachment))
        assertFalse(rig.owner.consent.state.value.foreground)
        assertEquals(CameraLens.FRONT, rig.binding.selected)
        assertFalse(rig.owner.requestLegacy())
        rig.owner.clear()
        assertNull(rig.binding.selected)
        assertNull(rig.services.last())
        try { rig.owner.attach(rig.launcher()::launch); fail("T961 retired owner revived") }
        catch (_: IllegalStateException) {}
    }

    @Test fun t961DiscardedActionCannotFinishReplacementEvenWithSameRequest() = Rig().use { rig ->
        val published = mutableListOf<LocalPermissionPrompt>()
        rig.owner.attach { published.add(it); true }
        rig.invitations.value = endpoint
        val old = published.single().action
        val original = rig.binding.permissionPending
        rig.owner.attach { published.add(it); true }
        val latest = published.last().action
        assertFalse(old.current())
        rig.allowed = true
        old.complete(true)
        assertSame(original, rig.binding.permissionPending)
        assertTrue(rig.captures.isEmpty())
        assertTrue(rig.prompts.result(published.last(), true))
        assertFalse(latest.current())
        assertEquals(listOf(CameraLens.FRONT), rig.captures)
        latest.complete(false)
        assertEquals(CameraLens.FRONT, rig.binding.selected)
    }

    @Test fun t944PermissionLossBeforeBackgroundStopRetiresCaptureWithoutNewInvitation() = Rig().use { rig ->
        rig.allowed = true
        val attachment = rig.owner.attach(rig.launcher()::launch)
        rig.invitations.value = endpoint.copy(background = true)
        assertEquals(CameraLens.FRONT, rig.binding.selected)
        rig.allowed = false
        assertTrue(rig.owner.detach(attachment))
        assertNull("T944 revoked camera survived Activity stop", rig.binding.selected)
        assertNull(rig.services.last())
    }

    @Test fun t944ForegroundReturnRechecksExistingBackgroundCapturePermission() = Rig().use { rig ->
        rig.allowed = true
        val attachment = rig.owner.attach(rig.launcher()::launch)
        rig.invitations.value = endpoint.copy(background = true)
        rig.owner.detach(attachment)
        assertEquals(CameraLens.FRONT, rig.binding.selected)
        rig.allowed = false
        var prompts = 0
        rig.owner.attach { prompts++; true }
        assertNull("T944 foreground return retained revoked native camera", rig.binding.selected)
        assertNull(rig.services.last())
        assertEquals(0, prompts) // Revocation ends the old run; no relabeled dialog.
    }

    @Test fun t961DeniedFailedAndUnavailableLaunchersCannotStartCapture() = Rig().use { rig ->
        rig.binding.start()
        rig.invitations.value = endpoint
        assertNull(rig.binding.permissionPending)
        rig.owner.attach { false }
        rig.binding.choose(CameraLens.FRONT)
        assertNull(rig.binding.permissionPending)
        rig.owner.attach(rig.launcher { error("SDK stopped") }::launch)
        rig.binding.choose(CameraLens.REAR)
        assertNull(rig.binding.permissionPending)
        val sdk = rig.launcher()
        rig.owner.attach(sdk::launch)
        rig.binding.choose(CameraLens.FRONT)
        assertTrue(sdk.complete(false))
        assertTrue(rig.captures.isEmpty())
        rig.binding.choose(CameraLens.FRONT)
        assertTrue(sdk.complete(true)) // reported grant without fresh OS permission
        assertTrue(rig.captures.isEmpty())
    }
}
