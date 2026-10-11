package com.blent

import org.junit.Assert.*
import org.junit.Test

class CameraPermissionLauncherTest {
    private val action = object : CameraPermissionAction {
        override fun current() = true
        override fun complete(granted: Boolean) {}
    }
    private fun prompt() = CameraPermissionPrompt(CameraPermissionAttachment(), action)

    @Test fun t961SdkSlotRetainsOriginalUntilActualReplyAndRejectsOverlaps() {
        val replies = mutableListOf<Pair<CameraPermissionPrompt, Boolean>>()
        var launches = 0
        val sdk = CameraPermissionLauncher({ launches++ }) { prompt, granted -> replies.add(prompt to granted) }
        val first = prompt()
        assertTrue(sdk.launch(first))
        assertFalse("T961 logical cancellation cannot free outstanding SDK slot", sdk.launch(prompt()))
        assertEquals(1, launches)
        assertTrue(sdk.complete(false))
        assertSame(first, replies.single().first)
        assertFalse(sdk.complete(true))
        val next = prompt()
        assertTrue(sdk.launch(next)); assertTrue(sdk.complete(true))
        assertEquals(next to true, replies.last())
        sdk.retire(); sdk.retire()
        assertFalse(sdk.launch(prompt())); assertFalse(sdk.complete(true))
    }

    @Test fun t961SynchronousCompletionAndSdkFailureDoNotLeakSlots() {
        var shouldThrow = true
        var complete = false
        val replies = mutableListOf<CameraPermissionPrompt>()
        lateinit var sdk: CameraPermissionLauncher
        sdk = CameraPermissionLauncher({
            if (shouldThrow) error("stopped SDK")
            if (complete) sdk.complete(true)
        }) { prompt, _ -> replies.add(prompt) }
        try { sdk.launch(prompt()); fail("T961 SDK failure hidden") }
        catch (_: IllegalStateException) {}
        assertFalse(sdk.complete(true))
        shouldThrow = false; complete = true
        val original = prompt(); assertTrue(sdk.launch(original)); assertSame(original, replies.single())
        assertFalse(sdk.complete(false))
        complete = false; assertTrue(sdk.launch(prompt()))
        sdk.retire(); assertFalse(sdk.complete(true))
    }

    @Test fun t961PublisherFailureCannotConsumeNextResultOrKeepDestroyedPrompt() {
        val sdk = CameraPermissionLauncher({}) { _, _ -> error("publisher failed") }
        assertTrue(sdk.launch(prompt()))
        try { sdk.complete(true); fail("T961 publisher failure hidden") }
        catch (_: IllegalStateException) {}
        assertFalse(sdk.complete(false))
        assertTrue(sdk.launch(prompt()))
        sdk.retire(); assertFalse(sdk.launch(prompt()))
    }
}
