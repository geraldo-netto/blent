package com.blent

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.ui.Modifier
import androidx.compose.ui.semantics.SemanticsActions
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.createComposeRule
import com.blent.connection.*
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
class MediaControlsReconnectTest {
    @get:Rule val compose = createComposeRule()
    private val names = listOf("camera", "microphone", "speakers")

    private fun render(rig: MediaControlsReconnectFixture) {
        compose.setContent {
            BlentTheme {
                Column(Modifier.verticalScroll(rememberScrollState())) {
                    CameraControls(rig.camera, rig.cameraPolicy)
                    AudioControls(rig.microphone, rig.microphonePolicy)
                    AudioControls(rig.speakers, rig.speakerPolicy)
                }
            }
        }
    }

    private fun click(action: String, index: Int) {
        compose.onNodeWithText("$action ${names[index]}").performScrollTo().assertIsEnabled().performClick()
    }

    private fun queued(action: String, index: Int): () -> Boolean = compose
        .onNodeWithText("$action ${names[index]}").performScrollTo().fetchSemanticsNode()
        .config[SemanticsActions.OnClick].action!!

    private fun start(rig: MediaControlsReconnectFixture, worker: MediaControlsReconnectFixture.Transport) {
        names.indices.forEach { click("Start", it) }
        compose.runOnIdle { assertTrue("T1242 consent alone acquired native media", rig.runs.all { it.isEmpty() }) }
        compose.runOnIdle { rig.admit(worker) }
        compose.waitUntil { rig.runs.all { it.size == 1 } }
    }

    @Test fun t1242RenderedStopsKeepSiblingConsumersAndRequireOriginalRetirement() {
        MediaControlsReconnectFixture().use { rig ->
            val worker = rig.connect()
            render(rig); start(rig, worker)
            for (index in listOf(2, 0, 1)) verifyStop(rig, worker, index)
            compose.runOnIdle {
                assertTrue(rig.connection.state.value.authenticated)
                assertNull(rig.connection.state.value.display)
                assertEquals(3, rig.terminalCommands(worker).size)
            }
        }
    }

    private fun verifyStop(rig: MediaControlsReconnectFixture, worker: MediaControlsReconnectFixture.Transport, index: Int) {
        val original = rig.runs[index].single()
        val previouslyRetired = rig.terminalCommands(worker).size
        click("Stop", index)
        compose.waitUntil { original.cancellations.get() > 0 }
        compose.runOnIdle {
            rig.closed(worker, index)
            assertFalse(original.retired)
            assertEquals(previouslyRetired, rig.terminalCommands(worker).size)
            for (sibling in rig.runs.flatten().filter { it !== original && !it.retired }) {
                assertEquals("T1242 Stop crossed sibling ownership", 0, sibling.cancellations.get())
            }
        }
        compose.runOnIdle { original.released.complete(Unit) }
        compose.waitUntil { rig.terminalCommands(worker).size == previouslyRetired + 1 }
        compose.onNodeWithText("Stop ${names[index]}").performScrollTo().assertIsNotEnabled()
    }

    @Test fun t1242ReconnectRequiresFreshUiConsentAndOriginalNativeCompletion() {
        MediaControlsReconnectFixture().use { rig ->
            val original = rig.connect()
            render(rig)
            val oldStarts = names.indices.map { queued("Start", it) }
            start(rig, original)
            val oldStops = names.indices.map { queued("Stop", it) }
            compose.runOnIdle { rig.connection.connect(ConnectionWorkerSpec("T1242 replacement")) }
            compose.runOnIdle { assertEquals(1, rig.transports.size); original.released.complete(Unit) }
            compose.waitUntil { rig.transports.size == 2 }
            val replacement = rig.transports.last()
            compose.runOnIdle { rig.offer(replacement); oldStarts.forEach { it() }; oldStops.forEach { it() } }
            names.forEach {
                compose.onNodeWithText("Start $it").performScrollTo().assertIsEnabled()
                compose.onNodeWithText("Stop $it").performScrollTo().assertIsNotEnabled()
            }
            compose.runOnIdle { assertTrue(replacement.commands.isEmpty()) }
            names.indices.forEach { click("Start", it) }
            compose.runOnIdle { rig.admit(replacement); assertTrue(rig.runs.all { it.size == 1 }) }
            for (index in listOf(1, 2, 0)) releaseOriginal(rig, replacement, index)
            compose.runOnIdle {
                val commands = replacement.commands.toList()
                oldStarts.forEach { it() }; oldStops.forEach { it() }
                assertEquals("T1242 old tap changed an active replacement", commands, replacement.commands.toList())
                rig.runs.forEach { assertEquals(0, it.last().cancellations.get()) }
            }
            for (index in names.indices) verifyReplacementStop(rig, replacement, index)
        }
    }

    private fun releaseOriginal(rig: MediaControlsReconnectFixture, replacement: MediaControlsReconnectFixture.Transport, index: Int) {
        val original = rig.runs[index].first()
        compose.runOnIdle { assertFalse(original.retired); original.released.complete(Unit) }
        compose.waitUntil { rig.runs[index].size == 2 }
        compose.runOnIdle {
            assertTrue(original.retired)
            assertTrue(rig.runs[index].last().started)
            assertEquals(0, rig.runs[index].last().cancellations.get())
            assertTrue("T1242 old completion reached replacement", rig.terminalCommands(replacement).isEmpty())
            val current = statuses(rig)[index]
            original.report("T1242 stale original status")
            assertEquals(current, statuses(rig)[index])
        }
    }

    private fun statuses(rig: MediaControlsReconnectFixture) =
        listOf(rig.camera.status, rig.microphone.status, rig.speakers.status)

    private fun verifyReplacementStop(rig: MediaControlsReconnectFixture, replacement: MediaControlsReconnectFixture.Transport, index: Int) {
        click("Stop", index)
        compose.runOnIdle { rig.closed(replacement, index); rig.runs[index].last().released.complete(Unit) }
        compose.waitUntil { rig.runs[index].last().retired }
        compose.runOnIdle { assertTrue(rig.connection.state.value.authenticated) }
    }

    @Test fun t1242DeniedMicrophonePermissionCannotStopCameraOrSpeakers() {
        MediaControlsReconnectFixture().use { rig ->
            rig.microphonePermission = false
            val worker = rig.connect()
            render(rig)
            names.indices.forEach { click("Start", it) }
            compose.runOnIdle {
                worker.report(ConnectionWorkerEvent.Camera(cameraRoute()))
                worker.report(ConnectionWorkerEvent.AudioRoute(audioRoute(ConnectionManagement.Resource.SPEAKERS)))
                val route = audioRoute(ConnectionManagement.Resource.MICROPHONE)
                worker.report(ConnectionWorkerEvent.AudioOffered(LocalAudioOffer(route.lease, route.profile, true)))
            }
            compose.waitUntil { rig.requested.size == 1 }
            compose.runOnIdle {
                assertTrue(rig.prompts.result(rig.requested.single(), false))
                assertTrue(rig.runs[1].isEmpty())
                for (index in listOf(0, 2)) {
                    assertTrue(rig.runs[index].single().started)
                    assertEquals(0, rig.runs[index].single().cancellations.get())
                }
                assertTrue(rig.connection.state.value.authenticated)
            }
            compose.onNodeWithText("Start microphone").performScrollTo().assertIsEnabled()
            compose.onNodeWithText("Stop microphone").performScrollTo().assertIsNotEnabled()
            listOf(0, 2).forEach { verifyStop(rig, worker, it) }
        }
    }
}
