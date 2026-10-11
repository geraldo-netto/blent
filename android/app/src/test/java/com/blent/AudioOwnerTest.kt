package com.blent

import android.Manifest
import kotlinx.coroutines.*
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.*
import org.robolectric.annotation.Config
import java.net.ServerSocket

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [27, 34])
class AudioOwnerTest {
    @org.junit.Before fun workerBridge() { WorkerTestBridge.install() }
    @Test fun t718_defaultOwnerRequestsPermissionAndRetiresBackgroundService() = runBlocking {
        val app = RuntimeEnvironment.getApplication()
        AudioOwner.reset()
        try {
            val prompts = mutableListOf<LocalPermissionPrompt>()
            val binding = AudioOwner.get(app); assertSame(binding, AudioOwner.get(app))
            AudioOwner.get(app, 2)
            AudioOwner.attach { prompts.add(it); true }
            binding.start(); org.robolectric.Shadows.shadowOf(android.os.Looper.getMainLooper()).idle()
            AudioInvitations.microphone.value = AudioEndpoint("a".repeat(64), 12345, 1, 1, 40, false)
            org.robolectric.Shadows.shadowOf(android.os.Looper.getMainLooper()).idle()
            assertEquals(1, prompts.size); assertTrue(AudioOwner.permissionResult(prompts.single(), false))
            Shadows.shadowOf(app).grantPermissions(Manifest.permission.RECORD_AUDIO)
            binding.configure(AudioPreferences(background = true, builtIn = false))
            ServerSocket(0).use { server ->
                AudioInvitations.microphone.value = AudioEndpoint("a".repeat(64), server.localPort, 1, 1, 40, true)
                Shadows.shadowOf(android.os.Looper.getMainLooper()).idle()
                withContext(Dispatchers.IO) { server.soTimeout = 2000; server.accept().use { socket ->
                    socket.soTimeout = 2000; assertEquals(76, socket.getInputStream().readNBytes(76).size)
                } }
                AudioOwner.stopSharing(); assertFalse(binding.sharing)
            }
            AudioOwner.stopped("old"); assertFalse(AudioOwner.owns("old"))
        } finally { AudioOwner.reset(); Shadows.shadowOf(android.os.Looper.getMainLooper()).idle() }
    }

    @Test fun t951UninitializedDetachedAndRejectedSdkOwnersFailClosed() {
        AudioOwner.reset()
        val app = RuntimeEnvironment.getApplication()
        Shadows.shadowOf(app).denyPermissions(Manifest.permission.RECORD_AUDIO)
        try {
            assertFalse(AudioOwner.detach(LocalPermissionAttachment()))
            assertThrows(IllegalStateException::class.java) { AudioOwner.attach { true } }
            for (direction in listOf(Int.MIN_VALUE, -1, 0, 3, Int.MAX_VALUE)) {
                assertThrows(IllegalStateException::class.java) { AudioOwner.get(app, direction) }
            }
            val binding = AudioOwner.get(app)
            assertThrows(IllegalStateException::class.java) { AudioOwner.attach { true } }
            binding.start()
            AudioInvitations.microphone.value = AudioEndpoint("ab".repeat(32), 12345, 1, 1, 40, false)
            Shadows.shadowOf(android.os.Looper.getMainLooper()).idle()
            assertNull(binding.permissionPending); assertFalse(binding.sharing)
            AudioOwner.get(app, 2)
            val attached = AudioOwner.attach { false }
            AudioInvitations.microphone.value = AudioEndpoint("ab".repeat(32), 12346, 1, 1, 40, false)
            Shadows.shadowOf(android.os.Looper.getMainLooper()).idle()
            assertNull(binding.permissionPending); assertFalse(binding.sharing)
            assertTrue(AudioOwner.detach(attached)); assertFalse(AudioOwner.detach(attached))
            val stale = LocalPermissionPrompt(attached, object : LocalPermissionAction {
                override fun current() = true
                override fun complete(granted: Boolean) { fail("T951 foreign SDK prompt granted capture") }
            })
            assertFalse(AudioOwner.permissionResult(stale, true))
            AudioOwner.reset(); assertFalse(AudioOwner.permissionResult(stale, false))
        } finally { AudioOwner.reset(); Shadows.shadowOf(android.os.Looper.getMainLooper()).idle() }
    }
}
