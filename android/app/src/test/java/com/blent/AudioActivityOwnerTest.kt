package com.blent

import kotlinx.coroutines.*
import kotlinx.coroutines.flow.MutableStateFlow
import org.junit.Assert.*
import org.junit.Test

class AudioActivityOwnerTest {
    private val endpoint = AudioEndpoint("ab".repeat(32), 12345, 1, 1, 40, false)
    private inner class Rig : AutoCloseable {
        val scope = CoroutineScope(SupervisorJob() + Dispatchers.Unconfined)
        val microphoneInvitations = MutableStateFlow<AudioEndpoint?>(null)
        val speakerInvitations = MutableStateFlow<AudioEndpoint?>(null)
        val prompts = LocalPermissionPrompts()
        var allowed = false
        val captures = mutableListOf<AudioEndpoint>()
        val services = mutableListOf<String?>()
        lateinit var owner: AudioActivityOwner
        private fun binding(direction: Int) = AudioBinding(AudioPreferences(background = true),
            { owner.requestPermission() }, { direction == 2 || allowed }, { value, _, _ ->
                object : LifecycleRun {
                    override suspend fun execute() { captures.add(value); awaitCancellation() }
                    override fun cancel() {}
                    override suspend fun retire() {}
                }
            }, if (direction == 1) microphoneInvitations else speakerInvitations, scope,
            services::add, direction, {})
        val microphone = binding(1)
        val speakers = binding(2)
        init { owner = AudioActivityOwner(microphone, speakers, prompts) }
        fun launcher(native: () -> Unit = {}) = LocalPermissionLauncher(native) { prompt, granted -> prompts.result(prompt, granted) }
        override fun close() { owner.clear(); scope.cancel() }
    }

    @Test fun t951ExistingInvitationOnAttachKeepsOneOriginalSdkRequest(): Unit = Rig().use { rig ->
        var launches = 0
        val sdk = rig.launcher { launches++ }
        rig.microphoneInvitations.value = endpoint
        rig.owner.attach(sdk::launch)
        assertNotNull(rig.microphone.permissionPending); assertEquals(1, launches)
        rig.allowed = true; assertTrue(sdk.complete(true))
        assertEquals(listOf(endpoint), rig.captures)
    }

    @Test fun t951OldDetachAndSdkResultCannotConsumeCurrentRequest(): Unit = Rig().use { rig ->
        val oldSdk = rig.launcher(); val old = rig.owner.attach(oldSdk::launch)
        rig.microphoneInvitations.value = endpoint
        val original = rig.microphone.permissionPending!!
        val nextSdk = rig.launcher(); val latest = rig.owner.attach(nextSdk::launch)
        assertFalse(rig.owner.detach(old))
        rig.allowed = true
        assertTrue(oldSdk.complete(true)); assertSame(original, rig.microphone.permissionPending)
        assertTrue(rig.captures.isEmpty())
        assertTrue(nextSdk.complete(true)); assertEquals(listOf(endpoint), rig.captures)
        assertTrue(rig.owner.detach(latest)); assertFalse(rig.microphone.sharing)
        assertFalse(rig.owner.detach(latest)); assertFalse(rig.owner.requestPermission())
    }

    @Test fun t951BusySdkSlotNeverRelabelsGrantAndRejectedLaunchClosesOriginal(): Unit = Rig().use { rig ->
        var launches = 0
        val sdk = rig.launcher { launches++ }
        rig.owner.attach(sdk::launch)
        rig.microphoneInvitations.value = endpoint
        rig.microphoneInvitations.value = endpoint.copy(port = 12346)
        assertEquals(1, launches); assertNull(rig.microphone.permissionPending)
        rig.allowed = true; assertTrue(sdk.complete(true)); assertTrue(rig.captures.isEmpty())
        rig.microphoneInvitations.value = endpoint
        assertEquals(listOf(endpoint), rig.captures)
    }

    @Test fun t951DiscardedActionAndFalseGrantCannotStartCapture(): Unit = Rig().use { rig ->
        val published = mutableListOf<LocalPermissionPrompt>()
        rig.owner.attach { published.add(it); true }
        rig.microphoneInvitations.value = endpoint
        val old = published.single().action
        val original = rig.microphone.permissionPending
        rig.owner.attach { published.add(it); true }
        old.complete(true); assertFalse(old.current()); assertSame(original, rig.microphone.permissionPending)
        assertTrue(rig.prompts.result(published.last(), true))
        assertTrue("T951 SDK boolean cannot replace fresh permission", rig.captures.isEmpty())
        rig.allowed = true
        published.last().action.complete(false); assertTrue(rig.captures.isEmpty())
    }

    @Test fun t951DetachKeepsOnlyExplicitBackgroundRunsAndClearRetiresOwner(): Unit = Rig().use { rig ->
        rig.allowed = true
        val attached = rig.owner.attach(rig.launcher()::launch)
        rig.microphoneInvitations.value = endpoint.copy(background = true)
        rig.speakerInvitations.value = endpoint.copy(direction = 2)
        assertTrue(rig.microphone.sharing); assertTrue(rig.speakers.sharing)
        assertTrue(rig.owner.detach(attached)); assertTrue(rig.microphone.sharing); assertFalse(rig.speakers.sharing)
        rig.owner.clear(); assertFalse(rig.microphone.sharing); assertNull(rig.services.last())
        assertThrows(IllegalStateException::class.java) { rig.owner.attach(rig.launcher()::launch) }
    }

    @Test fun t951LaunchFailureOrAbsentForegroundDeniesPermissionWithoutNewCapture(): Unit = Rig().use { rig ->
        rig.owner.attach { throw IllegalStateException("SDK unavailable") }
        rig.microphoneInvitations.value = endpoint
        assertNull(rig.microphone.permissionPending); assertTrue(rig.captures.isEmpty())
        rig.microphone.stop(); rig.microphone.start()
        rig.owner.clear()
        assertFalse(rig.owner.requestPermission())
    }

    @Test fun t951ReplacementAttachmentKeepsBothIndependentRunsAlive(): Unit = Rig().use { rig ->
        rig.allowed = true
        val old = rig.owner.attach(rig.launcher()::launch)
        rig.microphoneInvitations.value = endpoint
        rig.speakerInvitations.value = endpoint.copy(direction = 2)
        val latest = rig.owner.attach(rig.launcher()::launch)
        assertFalse(rig.owner.detach(old))
        assertTrue(rig.microphone.sharing); assertTrue(rig.speakers.sharing)
        assertEquals(listOf(1, 2), rig.captures.map { it.direction })
        assertTrue(rig.owner.detach(latest))
        assertFalse(rig.microphone.sharing); assertFalse(rig.speakers.sharing)
    }

    @Test fun t951BoundedPermissionAndLifecyclePropertyRequiresEveryOriginalProof() {
        for (flags in 0..15) Rig().use { rig ->
            val sdk = rig.launcher(); val attachment = rig.owner.attach(sdk::launch)
            val background = flags and 4 != 0
            rig.microphoneInvitations.value = endpoint.copy(background = background)
            val granted = flags and 1 != 0
            rig.allowed = flags and 2 != 0
            val detached = flags and 8 != 0
            if (detached) rig.owner.detach(attachment)
            assertTrue(sdk.complete(granted))
            val accepted = granted && rig.allowed && !detached
            assertEquals("T951 original permission/lifetime flags=$flags", accepted, rig.microphone.sharing)
            assertEquals(if (accepted) 1 else 0, rig.captures.size)
            if (!detached) rig.owner.detach(attachment)
            assertEquals(accepted && background, rig.microphone.sharing)
        }
    }
}
