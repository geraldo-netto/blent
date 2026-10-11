package com.blent

import kotlinx.coroutines.*
import kotlinx.coroutines.flow.MutableStateFlow
import org.junit.Assert.*
import org.junit.Test

class AudioPermissionIdentityTest {
    @Test fun t951LateResultCannotGrantOrDenyReplacementEvenWithReusedEndpoint() {
        val scope = CoroutineScope(SupervisorJob() + Dispatchers.Unconfined)
        val invitations = MutableStateFlow<AudioEndpoint?>(null)
        var permission = false; var opens = 0
        val binding = AudioBinding(AudioPreferences(), {}, { permission }, { _, _, _ ->
            object : LifecycleRun {
                override suspend fun execute() { opens++; awaitCancellation() }
                override fun cancel() {}
                override suspend fun retire() {}
            }
        }, invitations, scope, {}, 1, {})
        val endpoint = AudioEndpoint("ab".repeat(32), 12345, 1, 1, 40, false)
        try {
            binding.start(); invitations.value = endpoint
            val first = binding.permissionPending!!
            binding.stop(); binding.start(); invitations.value = endpoint
            val second = binding.permissionPending!!
            assertNotSame(first, second); assertSame(first.endpoint, second.endpoint)
            permission = true
            binding.permissionResult(true, first)
            assertEquals("T951 stale grant started replacement", 0, opens)
            assertSame(second, binding.permissionPending)
            binding.permissionResult(false, first)
            assertSame("T951 stale denial consumed replacement", second, binding.permissionPending)
            binding.permissionResult(true, second)
            assertEquals(1, opens); assertNull(binding.permissionPending)
            binding.permissionResult(false, second)
            assertTrue(binding.sharing)
        } finally { binding.shutdown(); scope.cancel() }
    }
}
