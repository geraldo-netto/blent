package com.blent

import kotlinx.coroutines.*
import kotlinx.coroutines.flow.MutableStateFlow
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.RuntimeEnvironment
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [27, 34])
class CameraWaitStatusTest {
    @Test fun t806_adapterPublishesNativeCallbackWaitWithoutExposingResourcesToPolicy() = runBlocking {
        val scope = CoroutineScope(SupervisorJob() + Dispatchers.Unconfined)
        val commands = MutableStateFlow<CameraEndpoint?>(null)
        val binding = CameraBinding(RuntimeEnvironment.getApplication(), { 0 }, {}, { true },
            { _, _, _, resources ->
                val finish = resources.startup()
                try { awaitCancellation() } finally { finish() }
            }, commands, scope)
        try {
            binding.start()
            commands.value = CameraEndpoint("a".repeat(64), 1234, 640, 480, 30, 1000, CameraLens.FRONT)
            withTimeout(2000) { while (!binding.status.contains("startup pending")) delay(1) }
            assertEquals("Camera startup pending.", binding.status)
            binding.choose(null)
            assertNull(binding.selected)
        } finally {
            binding.shutdown(); scope.cancel()
            withTimeout(2000) { scope.coroutineContext[Job]!!.join() }
        }
    }
}
