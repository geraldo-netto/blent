package com.blent.connection

import androidx.compose.foundation.layout.Column
import androidx.compose.runtime.*
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.createComposeRule
import androidx.compose.ui.semantics.SemanticsActions
import com.blent.BlentTheme
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.annotation.Config
import org.robolectric.annotation.LooperMode

@OptIn(ExperimentalTestApi::class)
@RunWith(com.blent.ComposeFixtureRunner::class)
@Config(sdk = [27, 34], qualifiers = "w960dp-h900dp-land")
@LooperMode(LooperMode.Mode.PAUSED)
class SavedComputerControlsTest {
    @get:Rule val compose = createComposeRule()
    private fun render(f: ConnectionUiFixture) {
        val configured = ConfiguredConnection(f.binding, f.preferences, f.scope)
        compose.setContent { BlentTheme { Column { ConnectionControls(f.binding, configured, f.saved) } } }
    }
    private fun open() { compose.onNodeWithText("Saved computers").performClick() }
    private fun select() { compose.onNodeWithText("Forget ${savedPeer(1).fingerprint}").performClick() }

    @Test fun t1169ExplicitOfflineForgetShowsFullFingerprintAndReleasesOnlyAfterCleanup() {
        ConnectionUiFixture().use { f ->
            render(f); open()
            compose.onNodeWithText("Connect USB").assertIsNotEnabled()
            compose.onNodeWithText("Reading or updating saved computers…").assertExists()
            compose.runOnIdle { f.savedRuns.single().complete() }
            select()
            compose.onNodeWithText("Forget this computer?").assertExists()
            compose.onNodeWithText(savedPeer(1).fingerprint).assertExists()
            compose.onNodeWithText("Keep computer").performClick()
            compose.runOnIdle { assertEquals(listOf(SavedTrustRequest.List), f.savedRequests) }
            select(); compose.onNodeWithText("Confirm forget computer").performClick()
            compose.runOnIdle {
                assertEquals(SavedTrustRequest.Forget(SavedComputerSelection(savedPeer(0), 1, savedPeer(1))), f.savedRequests.last())
                f.savedRuns.last().complete(0, retire = false)
            }
            compose.onNodeWithText("Waiting for saved-computer access to close…").assertExists()
            compose.onNodeWithText("Refresh saved computers").assertIsNotEnabled()
            compose.onNodeWithText("Connect USB").assertIsNotEnabled()
            compose.runOnIdle { f.savedRuns.last().release.complete(Unit) }
            compose.onNodeWithText("No saved computers.").assertExists()
            compose.onNodeWithText("Close saved computers").performClick()
            compose.onNodeWithText("Connect USB").assertIsEnabled()
            compose.runOnIdle { assertTrue(f.workers.isEmpty()) }
        }
    }
    @Test fun t1169LiveOrRetiringConnectionBlocksRefreshAndForgettingWithoutOpeningStore() {
        ConnectionUiFixture().use { f ->
            render(f)
            compose.onNodeWithText("Connect USB").performClick(); open()
            compose.onNodeWithText("Refresh to load saved computers.").assertExists()
            compose.onNodeWithText("Refresh saved computers").assertIsNotEnabled()
            compose.onNodeWithText("Disconnect USB and wait for cleanup before refreshing or forgetting.").assertExists()
            compose.runOnIdle { assertTrue(f.savedRuns.isEmpty()); f.binding.disconnect() }
            compose.onNodeWithText("Refresh saved computers").assertIsNotEnabled()
            compose.runOnIdle { f.workers.single().released.complete(Unit) }
            compose.onNodeWithText("Refresh saved computers").assertIsEnabled().performClick()
            compose.runOnIdle { f.savedRuns.single().complete() }
            compose.onNodeWithText("Saved computers are ready.").assertExists()
            compose.onNodeWithText("Close saved computers").performClick()
            compose.onNodeWithText("Connect USB").performClick(); open()
            compose.onNodeWithText("Forget ${savedPeer(1).fingerprint}").assertIsNotEnabled()
        }
    }
    @Test fun t1169StopAndStorageErrorsHaveAccurateStatesAndNoImplicitRetry() {
        ConnectionUiFixture().use { f ->
            render(f); open()
            compose.onNodeWithText("Stop saved-computer access").performClick()
            compose.onNodeWithText("Waiting for saved-computer access to close…").assertExists()
            compose.runOnIdle { f.savedRuns.single().release.complete(Unit) }
            compose.onNodeWithText("Saved-computer access failed.").assertExists()
            compose.onNodeWithText("Refresh saved computers").performClick()
            compose.runOnIdle { f.savedRuns.last().apply {
                failure = IllegalStateException("store read failed"); release.complete(Unit); finish.complete(Unit)
            } }
            compose.onNodeWithText("store read failed").assertExists()
        }
    }
    @Test fun t1169DisposalRevokesOldConfirmationButKeepsAcceptedWorkForReplacementView() {
        ConnectionUiFixture().use { f ->
            var shown by mutableStateOf(true)
            compose.setContent { BlentTheme { if (shown) SavedComputerControls(f.saved) } }
            open(); compose.runOnIdle { f.savedRuns.single().complete() }; select()
            val old = compose.runOnIdle {
                val viewer = checkNotNull(f.saved.state.value.viewer)
                f.saved.choice(viewer, savedPeer(1))!!
            }
            compose.runOnIdle { shown = false }
            compose.runOnIdle { assertNull(f.saved.state.value.viewer); shown = true }
            compose.onNodeWithText("Forget this computer?").assertDoesNotExist()
            open()
            compose.runOnIdle {
                val current = checkNotNull(f.saved.state.value.viewer)
                assertFalse(f.saved.forget(current, old)); assertEquals(1, f.savedRuns.size)
            }
            select(); compose.onNodeWithText("Confirm forget computer").performClick()
            compose.runOnIdle { shown = false }
            compose.runOnIdle {
                assertEquals(0, f.savedRuns.last().cancelled); assertEquals(2, f.savedRequests.size)
                shown = true
            }
            open(); compose.onNodeWithText("Reading or updating saved computers…").assertExists()
            compose.runOnIdle { f.savedRuns.last().complete(0) }
            compose.onNodeWithText("No saved computers.").assertExists()
        }
    }

    @Test fun t1169UnknownCleanupKeepsAccessBlockedInItsOriginalProcessDomain() = kotlinx.coroutines.runBlocking<Unit> {
        val scope = kotlinx.coroutines.CoroutineScope(kotlinx.coroutines.SupervisorJob() + kotlinx.coroutines.Dispatchers.Unconfined)
        val access = ConnectionAccess()
        val runs = mutableListOf<SavedTrustUiRun>()
        val binding = SavedTrustBinding(scope, access, SavedTrustRunFactory { _, report ->
            SavedTrustUiRun(report).also { runs.add(it) }
        })
        try {
            compose.setContent { BlentTheme { SavedComputerControls(binding) } }; open()
            compose.runOnIdle { runs.single().apply {
                closeFailure = IllegalStateException("close unknown"); complete(0)
            } }
            compose.onNodeWithText("Saved-computer cleanup is unconfirmed; access remains blocked.").assertExists()
            compose.onNodeWithText("Refresh saved computers").assertIsNotEnabled()
            compose.runOnIdle { assertEquals(ConnectionUse.SAVED_TRUST, access.current()) }
        } finally {
            runs.forEach { it.release.complete(Unit); it.finish.complete(Unit) }
            scope.coroutineContext[kotlinx.coroutines.Job]!!.cancel()
            scope.coroutineContext[kotlinx.coroutines.Job]!!.join()
        }
        // No native resource exists in this fault fixture; its isolated heap
        // domain retains the simulated unknown permit through scope loss.
        assertEquals(ConnectionUse.SAVED_TRUST, access.current())
    }

    @Test fun t1169BoundedLazyListKeepsFirstAndLastPeerIdentityAcrossSnapshotReplacement() {
        var selected: ConnectionPeer? = null
        var view by mutableStateOf(SavedTrustView(phase = SavedTrustPhase.READY,
            computers = SavedComputers(savedPeer(0), 1, (1..1024).map(::savedPeer))))
        compose.setContent { BlentTheme { SavedComputerList(view, true) { selected = it } } }
        compose.onNodeWithText("Forget ${savedPeer(1).fingerprint}").performClick()
        compose.runOnIdle { assertEquals(savedPeer(1), selected) }
        compose.onNode(hasScrollToIndexAction()).performScrollToIndex(1023)
        compose.onNodeWithText("Forget ${savedPeer(1024).fingerprint}").performClick()
        compose.runOnIdle {
            assertEquals(savedPeer(1024), selected)
            view = SavedTrustView(phase = SavedTrustPhase.READY,
                computers = SavedComputers(savedPeer(0), 2, listOf(savedPeer(2048))))
        }
        compose.onNodeWithText("Forget ${savedPeer(2048).fingerprint}").performClick()
        compose.runOnIdle { assertEquals(savedPeer(2048), selected) }
        compose.runOnIdle { view = SavedTrustView(phase = SavedTrustPhase.READY) }
        compose.onAllNodesWithText("Forget", substring = true).assertCountEquals(0)
    }

    @Test fun t1169KeepComputerRevokesTheOriginalCapturedConfirmation() {
        ConnectionUiFixture().use { f ->
            render(f); open(); compose.runOnIdle { f.savedRuns.single().complete() }; select()
            val stale = checkNotNull(compose.onNodeWithText("Confirm forget computer")
                .fetchSemanticsNode().config[SemanticsActions.OnClick].action)
            compose.onNodeWithText("Keep computer").performClick()
            compose.runOnIdle { stale(); assertEquals(1, f.savedRequests.size) }
        }
    }

    @Test fun t1169ClosingTheBrowserRevokesItsViewerAndLateConfirmation() {
        ConnectionUiFixture().use { f ->
            render(f); open(); compose.runOnIdle { f.savedRuns.single().complete() }; select()
            val stale = checkNotNull(compose.onNodeWithText("Confirm forget computer")
                .fetchSemanticsNode().config[SemanticsActions.OnClick].action)
            compose.onNodeWithText("Keep computer").performClick()
            compose.onNodeWithText("Close saved computers").performClick()
            compose.runOnIdle {
                assertNull(f.saved.state.value.viewer)
                stale(); assertEquals(1, f.savedRequests.size)
            }
        }
    }

    @Test fun t1169SupersededDismissAndConfirmCannotChangeTheNewSelection() {
        ConnectionUiFixture().use { f ->
            render(f); open(); compose.runOnIdle { f.savedRuns.single().complete(2) }; select()
            val oldConfirm = checkNotNull(compose.onNodeWithText("Confirm forget computer")
                .fetchSemanticsNode().config[SemanticsActions.OnClick].action)
            val oldDismiss = checkNotNull(compose.onNodeWithText("Keep computer")
                .fetchSemanticsNode().config[SemanticsActions.OnClick].action)
            compose.onNodeWithText("Keep computer").performClick()
            compose.onNodeWithText("Forget ${savedPeer(2).fingerprint}").performClick()
            compose.runOnIdle { oldDismiss() }
            compose.onNodeWithText("Forget this computer?").assertExists()
            compose.onNodeWithText(savedPeer(2).fingerprint).assertExists()
            compose.runOnIdle { oldConfirm(); assertEquals(1, f.savedRequests.size) }
            compose.onNodeWithText("Confirm forget computer").performClick()
            compose.runOnIdle {
                assertEquals(SavedTrustRequest.Forget(SavedComputerSelection(savedPeer(0), 1, savedPeer(2))), f.savedRequests.last())
            }
        }
    }
}
