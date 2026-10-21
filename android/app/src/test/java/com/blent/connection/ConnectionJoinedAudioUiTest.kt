package com.blent.connection

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.ui.Modifier
import androidx.compose.ui.semantics.SemanticsActions
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.createComposeRule
import com.blent.*
import kotlinx.coroutines.*
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.annotation.Config
import org.robolectric.annotation.LooperMode

@RunWith(ComposeFixtureRunner::class)
@Config(sdk = [27, 34], qualifiers = "w960dp-h600dp-land",
    instrumentedPackages = ["androidx.compose.ui.platform"])
@LooperMode(LooperMode.Mode.PAUSED)
class ConnectionJoinedAudioUiTest {
    @get:Rule val compose = createComposeRule()

    private fun render(rig: JoinedAudioUiFixture) {
        compose.setContent {
            BlentTheme {
                Column(Modifier.verticalScroll(rememberScrollState())) {
                    AudioControls(rig.microphone.binding, rig.microphoneControls)
                    AudioControls(rig.speakers.binding, rig.speakerControls)
                }
            }
        }
    }

    private fun click(action: String) = compose.onNodeWithText(action).performScrollTo().assertIsEnabled().performClick()
    private fun queued(action: String): () -> Boolean = compose.onNodeWithText(action)
        .performScrollTo().fetchSemanticsNode().config[SemanticsActions.OnClick].action!!

    @Test fun t1245UiStopWaitsForOriginalReadAndBinderRetirementWhileSpeakerPcmContinues() = runBlocking {
        val rig = JoinedAudioUiFixture()
        try {
            rig.microphone.native.holdRead = true
            val display = rig.connect()
            rig.offerBoth(); render(rig)
            click("Start speakers")
            val speaker = rig.speakers.admitPrepared()
            rig.play()
            click("Start microphone")
            rig.original.until { rig.microphone.sdk.size == 1 }
            assertTrue(rig.microphone.approve(true))
            val microphone = rig.microphone.admitPrepared()
            rig.capture()
            val oldStop = queued("Stop microphone")
            click("Stop microphone")
            rig.original.until { rig.microphone.native.exitRequested }
            rig.microphone.native.release.complete(Unit)
            assertFalse(rig.microphone.native.cleanup.isCompleted)
            assertFalse(rig.terminal(microphone.lease))
            val rendered = rig.speakers.native.written.get()
            rig.feedSpeakers(4)
            rig.original.until { rig.speakers.native.written.get() > rendered }
            assertSame(display, rig.original.connection.state.value.display)
            assertSame(speaker, rig.speakers.selectedRoute()?.route)
            assertTrue(rig.speakers.binding.sharing)
            assertFalse(rig.speakers.native.exitRequested)
            rig.microphone.native.readRelease.complete(Unit)
            rig.original.until { rig.terminal(microphone.lease) }
            assertEquals(1, rig.microphone.native.unbound.get())
            assertEquals(1, rig.microphone.native.closes.get())
            compose.runOnIdle { oldStop() }
            assertFalse(rig.speakers.native.exitRequested)
            click("Stop speakers"); rig.retire(rig.speakers, speaker)
            assertSame(display, rig.original.connection.state.value.display)
            assertTrue(rig.original.connection.state.value.authenticated)
        } finally { rig.close() }
    }

    @Test fun t1245FreshSpeakerGenerationRequiresUiStartAndRejectsOldRenderedActions() = runBlocking {
        val rig = JoinedAudioUiFixture()
        try {
            val display = rig.connect()
            val firstOffer = rig.speakers.offer(request = 3)
            render(rig)
            val oldStart = queued("Start speakers")
            click("Start speakers")
            val route = rig.speakers.admitPrepared()
            rig.play()
            val oldStop = queued("Stop speakers")
            click("Stop speakers"); rig.retire(rig.speakers, route)
            rig.original.retirePeer(route.lease)
            rig.original.until { rig.original.link.messages().contains(
                ConnectionManagement.ResultMessage(route.lease.key, ConnectionManagement.Outcome.REVOKED)) }
            rig.original.replaceChannel(route.lease)
            val fresh = JoinedAudioWorkerFixture(2)
            rig.speakers.native.next = fresh
            val nextOffer = rig.speakers.offer(request = 4, generation = 2)
            assertNotSame("T1245 offer helper returned the old owner", firstOffer.owner, nextOffer.owner)
            assertFalse("T1245 replacement offer was already admitted", nextOffer.admitted)
            assertEquals(2L, rig.speakers.selectedOffer()?.lease?.key?.generation)
            compose.waitUntil(4000) { compose.onAllNodesWithText("Start speakers").fetchSemanticsNodes().isNotEmpty() }
            val originalControls = rig.original.link.controls.filterIsInstance<ConnectionWorkerCommand.AudioChoice>()
            compose.runOnIdle { oldStart(); oldStop() }
            assertEquals("T1245 old rendered actions changed replacement consent", originalControls,
                rig.original.link.controls.filterIsInstance<ConnectionWorkerCommand.AudioChoice>())
            assertTrue(fresh.tasks.isEmpty())
            compose.waitUntil(4000) { compose.onAllNodesWithText("Start speakers").fetchSemanticsNodes().isNotEmpty() }
            compose.onNodeWithText("Start speakers").performScrollTo().assertIsEnabled()
            click("Start speakers")
            val next = rig.speakers.admitPrepared()
            rig.original.until { fresh.tasks.size == 1 }
            rig.play(fresh, 152)
            compose.runOnIdle { oldStart(); oldStop() }
            assertFalse(fresh.exitRequested)
            assertNotEquals(route.lease, next.lease)
            assertSame(display, rig.original.connection.state.value.display)
            click("Stop speakers")
            rig.original.until { fresh.exitRequested }
            assertFalse(rig.terminal(next.lease))
            fresh.release.complete(Unit)
            rig.original.until { rig.terminal(next.lease) }
            assertEquals(1, fresh.unbound.get())
            assertTrue(rig.original.connection.state.value.authenticated)
        } finally { rig.close() }
    }

    @Test fun t1245UiMicrophoneDenialReachesManagementWithoutStoppingPlayingSpeaker() = runBlocking {
        val rig = JoinedAudioUiFixture()
        try {
            val display = rig.connect()
            rig.offerBoth(); render(rig)
            click("Start speakers")
            val speaker = rig.speakers.admitPrepared()
            rig.play()
            click("Start microphone")
            rig.original.until { rig.microphone.sdk.size == 1 }
            val lease = checkNotNull(rig.microphone.selectedOffer()).lease
            assertTrue(rig.microphone.approve(false))
            rig.original.until { rig.original.link.messages().contains(
                ConnectionManagement.ResultMessage(lease.key, ConnectionManagement.Outcome.PERMISSION_DENIED)) }
            assertEquals(0, rig.microphone.native.opened.get())
            assertTrue(rig.microphone.native.tasks.isEmpty())
            assertSame(display, rig.original.connection.state.value.display)
            assertSame(speaker, rig.speakers.selectedRoute()?.route)
            assertTrue(rig.speakers.binding.sharing)
            val rendered = rig.speakers.native.written.get()
            rig.feedSpeakers(4)
            rig.original.until { rig.speakers.native.written.get() > rendered }
            assertFalse(rig.speakers.native.exitRequested)
            click("Stop speakers"); rig.retire(rig.speakers, speaker)
        } finally { rig.close() }
    }
}
