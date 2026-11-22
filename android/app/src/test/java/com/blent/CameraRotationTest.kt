package com.blent

import kotlinx.coroutines.*
import kotlinx.coroutines.flow.MutableStateFlow
import org.junit.Assert.*
import org.junit.Test

/** Portable rotation policy: fake workers own explicit retirement gates. */
class CameraRotationTest {
    private class Run(val angle: Int, val closed: CompletableDeferred<Unit>) : LifecycleRun {
        var cancelled = false
        var retired = false
        override suspend fun execute(): Unit = awaitCancellation()
        override fun cancel() { cancelled = true }
        override suspend fun retire() { closed.await(); retired = true }
    }
    private class Rig(background: Boolean = false) : AutoCloseable {
        var angle = 0
        var allowed = true
        val scope = CoroutineScope(SupervisorJob() + Dispatchers.Unconfined)
        val runs = mutableListOf<Run>()
        val gate = CompletableDeferred<Unit>()
        val endpoint = CameraEndpoint("a".repeat(64), 12345, 640, 480, 30, 3000, background = background)
        val binding = CameraBinding({ angle }, {}, { allowed },
            { _, _, rotation, _ -> Run(rotation(), gate).also { runs.add(it) } },
            MutableStateFlow(endpoint), scope, {})
        init { binding.start(); binding.choose(CameraLens.FRONT) }
        override fun close() { binding.shutdown(); gate.complete(Unit); scope.cancel() }
    }

    @Test fun t1572RotationWaitsForOriginalRetirementAndCoalescesToLatestAngle() {
        Rig().use { rig ->
            assertEquals(listOf(0), rig.runs.map { it.angle })
            rig.angle = 1; rig.binding.rotationChanged()
            assertTrue("T1572 rotation did not retire original capture", rig.runs.single().cancelled)
            rig.angle = 3; rig.binding.rotationChanged()
            assertEquals(1, rig.runs.size)
            rig.gate.complete(Unit)
            assertTrue(rig.runs.first().retired)
            assertEquals(listOf(0, 3), rig.runs.map { it.angle })
            assertEquals(CameraLens.FRONT, rig.binding.selected)
        }
    }

    @Test fun t1572DuplicateInvalidAndStoppedRotationNeverOpenAWorker() {
        Rig().use { rig ->
            for (angle in listOf(0, -1, 4, Int.MIN_VALUE, Int.MAX_VALUE)) {
                rig.angle = angle; rig.binding.rotationChanged()
            }
            assertFalse(rig.runs.single().cancelled)
            rig.binding.choose(null)
            rig.angle = 2; rig.binding.rotationChanged()
            rig.gate.complete(Unit)
            assertEquals(1, rig.runs.size)
            assertTrue(rig.runs.single().retired)
            assertNull(rig.binding.selected)
        }
    }

    @Test fun t1572RotationCannotBypassFreshPermissionOrRestartAfterShutdown() {
        Rig().use { rig ->
            rig.allowed = false
            rig.angle = 2; rig.binding.rotationChanged()
            assertTrue("T1572 changed rotation left a permissionless run", rig.runs.single().cancelled)
            rig.binding.shutdown()
            rig.allowed = true; rig.binding.rotationChanged()
            rig.gate.complete(Unit)
            assertEquals(1, rig.runs.size)
            assertNull(rig.binding.selected)
        }
    }
    @Test fun t1572BackgroundCaptureKeepsFollowingDisplayWhileActivityIsStopped() {
        Rig(background = true).use { rig ->
            rig.binding.stop()
            assertEquals(CameraLens.FRONT, rig.binding.selected)
            rig.angle = 2; rig.binding.rotationChanged()
            assertTrue(rig.runs.single().cancelled)
            rig.gate.complete(Unit)
            assertEquals(listOf(0, 2), rig.runs.map { it.angle })
            assertEquals(CameraLens.FRONT, rig.binding.selected)
        }
    }

    @Test fun t1572InvalidInitialRotationRejectsBeforeWorkerCreation() {
        Rig().use { rig ->
            rig.binding.choose(null)
            rig.angle = -1
            rig.binding.choose(CameraLens.FRONT)
            rig.gate.complete(Unit)
            assertEquals(1, rig.runs.size)
            assertNull(rig.binding.selected)
            assertEquals("Invalid display rotation", rig.binding.status)
        }
    }

}
