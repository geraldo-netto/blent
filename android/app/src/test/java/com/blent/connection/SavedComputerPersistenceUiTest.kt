package com.blent.connection

import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.createComposeRule
import com.blent.BlentTheme
import com.blent.LifecycleRun
import java.nio.file.Files
import java.nio.file.attribute.PosixFilePermissions
import kotlinx.coroutines.*
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
class SavedComputerPersistenceUiTest {
    @get:Rule val compose = createComposeRule()

    @Test fun t1169ConfirmedUiForgetPersistsThroughRealStoreCloseAndFreshReopen() = runBlocking<Unit> {
        val directory = Files.createTempDirectory("t1169-saved-ui")
        Files.setPosixFilePermissions(directory, PosixFilePermissions.fromString("rwx------"))
        val scope = CoroutineScope(SupervisorJob() + Dispatchers.Default)
        val access = ConnectionAccess()
        val binding = SavedTrustBinding(scope, access, SavedTrustRunFactory { request, report ->
            val raw = SavedTrustWorker(request,
                { SavedTrustAccesses.open(AndroidPeerTrustStore.open(directory)) { savedPeer(0) } },
                { report((it as ConnectionWorkerEvent.SavedComputers).page) })
            object : LifecycleRun {
                override suspend fun execute() = withContext(Dispatchers.IO) { raw.execute() }
                override fun cancel() = raw.cancel()
                override suspend fun retire() = raw.retire()
            }
        })
        try {
            AndroidPeerTrustStore.open(directory).use {
                it.replace(ConnectionTrustCodec.encode(TrustSnapshot(savedPeer(0), 7, listOf(savedPeer(1), savedPeer(2)))))
            }
            compose.setContent { BlentTheme { SavedComputerControls(binding) } }
            compose.onNodeWithText("Saved computers").performClick()
            compose.waitUntil(5000) { binding.state.value.phase == SavedTrustPhase.READY }
            compose.onNodeWithText("Forget ${savedPeer(1).fingerprint}").performClick()
            compose.onNodeWithText(savedPeer(1).fingerprint).assertExists()
            compose.onNodeWithText("Confirm forget computer").performClick()
            compose.waitUntil(5000) { binding.state.value.phase == SavedTrustPhase.READY }
            assertNull(access.current())
            AndroidPeerTrustStore.open(directory).use {
                val persisted = ConnectionTrustCodec.decode(checkNotNull(it.load()))
                assertEquals(8L, persisted.revision); assertEquals(listOf(savedPeer(2)), persisted.peers)
            }
            compose.onNodeWithText("Forget ${savedPeer(1).fingerprint}").assertDoesNotExist()
            compose.onNodeWithText("Forget ${savedPeer(2).fingerprint}").assertExists()
            compose.onNodeWithText("Refresh saved computers").performClick()
            compose.waitUntil(5000) { binding.state.value.phase == SavedTrustPhase.READY }
            assertEquals(listOf(savedPeer(2)), binding.state.value.computers!!.peers)
        } finally {
            scope.cancel(); scope.coroutineContext[Job]!!.join()
            check(access.current() == null) { "Original file ownership unknown; preserve $directory" }
            Files.walk(directory).use { files -> files.sorted(Comparator.reverseOrder()).forEach(Files::delete) }
        }
    }
}
