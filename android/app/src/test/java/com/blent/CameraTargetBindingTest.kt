package com.blent

import kotlinx.coroutines.*
import kotlinx.coroutines.flow.MutableStateFlow
import org.junit.Assert.*
import org.junit.Test

class CameraTargetBindingTest {
    private val endpoint = CameraEndpoint("ab".repeat(32), 12345, 640, 480, 30, 1000, CameraLens.FRONT)
    private class Authority : CameraCaptureAuthority {
        var available = true
        var stops = 0
        var failure: Exception? = null
        val claims = mutableListOf<Claim>()
        var retainHook: () -> Unit = {}
        inner class Claim : CameraCaptureClaim {
            val proofs = mutableListOf<ResourceRetirement>()
            override fun current() = available
            override fun retired(proof: ResourceRetirement) { proofs.add(proof) }
        }
        override fun retain(): CameraCaptureClaim? {
            retainHook()
            return if (available) Claim().also { claims.add(it) } else null
        }
        override fun stop(): Boolean { stops++; available = false; failure?.let { throw it }; return true }
    }
    private class Worker(val endpoint: CameraEndpoint, val lens: CameraLens, val report: (String) -> Unit) : LifecycleRun {
        val release = CompletableDeferred<Unit>()
        var started = 0
        var retired = 0
        var failure: Exception? = null
        override suspend fun execute() { started++; awaitCancellation() }
        override fun cancel() {}
        override suspend fun retire() { release.await(); failure?.let { throw it }; retired++ }
    }
    private inner class Rig : AutoCloseable {
        val scope = CoroutineScope(SupervisorJob() + Dispatchers.Unconfined)
        val adb = MutableStateFlow<CameraEndpoint?>(null)
        val workers = mutableListOf<Worker>()
        val background = mutableListOf<String?>()
        var permission = true
        var prompts = 0
        val binding = CameraBinding({ 0 }, { prompts++ }, { permission }, { host, lens, _, report ->
            Worker(host, lens, report).also { workers.add(it) }
        }, adb, scope, { background.add(it) })
        fun target(authority: Authority, value: CameraEndpoint = endpoint) = CameraCaptureTarget(value, authority)
        override fun close() {
            workers.forEach { it.release.complete(Unit) }
            binding.shutdown(); scope.cancel()
        }
    }

    @Test fun t943UnselectedExternalTargetRemainsAvailableForExplicitLocalLens() {
        Rig().use { rig ->
            val authority = Authority()
            rig.binding.start()
            rig.binding.useTarget(rig.target(authority, endpoint.copy(requestedLens = null)))
            assertEquals("T943 absent host lens is not local stop", 0, authority.stops)
            assertNull(rig.binding.selected)
            rig.binding.choose(CameraLens.REAR)
            assertEquals(CameraLens.REAR, rig.workers.single().lens)
        }
    }

    @Test fun t943RefusedTargetCannotLeaveSelectedCameraWithoutNativeOwnership() {
        Rig().use { rig ->
            val authority = Authority().apply { available = false }
            rig.binding.start(); rig.binding.useTarget(rig.target(authority))
            assertTrue(rig.workers.isEmpty())
            assertTrue(authority.claims.isEmpty())
            assertNull("T943 failed admission must clear selected lens", rig.binding.selected)
        }
    }

    @Test fun t943LensReplacementKeepsEveryClaimUntilActualCleanupOrNoStart() {
        Rig().use { rig ->
            val authority = Authority()
            rig.binding.start(); rig.binding.useTarget(rig.target(authority))
            val first = rig.workers.single()
            rig.binding.choose(CameraLens.REAR)
            rig.binding.choose(CameraLens.FRONT)
            assertEquals(3, authority.claims.size)
            assertTrue(authority.claims[0].proofs.isEmpty())
            assertEquals(listOf(ResourceRetirement.NOT_STARTED), authority.claims[1].proofs)
            assertTrue(authority.claims[2].proofs.isEmpty())
            assertEquals(1, rig.workers.size)
            assertTrue(authority.available)
            first.report("stale old worker status")
            assertFalse(rig.binding.status.contains("stale"))
            first.release.complete(Unit)
            assertEquals(listOf(ResourceRetirement.RETIRED), authority.claims[0].proofs)
            assertEquals(2, rig.workers.size)
            assertEquals(CameraLens.FRONT, rig.workers.last().lens)
            rig.binding.choose(null)
            assertFalse(authority.available)
            assertTrue(authority.claims[2].proofs.isEmpty())
            rig.workers.last().release.complete(Unit)
            assertEquals(listOf(ResourceRetirement.RETIRED), authority.claims[2].proofs)
        }
    }

    @Test fun t943EqualMetadataReplacementAndAdbFallbackKeepOriginalClaims() {
        Rig().use { rig ->
            rig.binding.start()
            val first = Authority()
            val second = Authority()
            rig.binding.useTarget(rig.target(first))
            val original = rig.workers.single()
            rig.binding.useTarget(rig.target(second))
            assertFalse(first.available)
            assertTrue(first.claims.single().proofs.isEmpty())
            assertTrue(second.available)
            original.release.complete(Unit)
            assertEquals(listOf(ResourceRetirement.RETIRED), first.claims.single().proofs)
            assertEquals(2, rig.workers.size)
            rig.adb.value = endpoint.copy(port = 12346, requestedLens = CameraLens.REAR)
            assertEquals(endpoint, rig.binding.endpoint)
            val external = rig.workers.last()
            rig.binding.useTarget(null)
            assertEquals(12346, rig.binding.endpoint!!.port)
            assertEquals(2, rig.workers.size)
            assertTrue(second.claims.single().proofs.isEmpty())
            external.release.complete(Unit)
            assertEquals(listOf(ResourceRetirement.RETIRED), second.claims.single().proofs)
            assertEquals(CameraLens.REAR, rig.workers.last().lens)
            assertEquals(12346, rig.workers.last().endpoint.port)
        }
    }

    @Test fun t943FailedWorkerExitQuarantinesClaimsAndDoesNotOpenReplacement() {
        Rig().use { rig ->
            val first = Authority()
            rig.binding.start(); rig.binding.useTarget(rig.target(first))
            val worker = rig.workers.single().apply { failure = IllegalStateException("failed native exit") }
            rig.binding.choose(null)
            worker.release.complete(Unit)
            assertTrue(first.claims.single().proofs.isEmpty())
            val second = Authority()
            rig.binding.useTarget(rig.target(second))
            assertEquals(1, rig.workers.size)
            assertTrue(second.claims.isEmpty())
            assertFalse(second.available)
            assertTrue(rig.binding.status.contains("failed native exit"))
        }
    }

    @Test fun t943PermissionLossClosesExternalTargetWithoutLegacyPromptOrNativeFactory() {
        Rig().use { rig ->
            rig.permission = false
            val authority = Authority()
            rig.binding.start(); rig.binding.useTarget(rig.target(authority))
            assertFalse(authority.available)
            assertTrue(rig.workers.isEmpty())
            assertEquals(0, rig.prompts)
            rig.binding.permissionResult(true)
            assertTrue(rig.workers.isEmpty())
        }
    }
    @Test fun t943TargetBeforeForegroundStartsOnceAndKeepsBackgroundOwnership() {
        Rig().use { rig ->
            val authority = Authority()
            val target = rig.target(authority, endpoint.copy(background = true))
            rig.binding.useTarget(target)
            assertTrue(rig.workers.isEmpty())
            rig.binding.start(); rig.binding.start()
            assertEquals(1, rig.workers.size)
            assertEquals(1, authority.claims.size)
            rig.binding.stop()
            assertTrue(authority.available)
            assertEquals(CameraLens.FRONT, rig.binding.selected)
            rig.binding.start()
            assertEquals(1, rig.workers.size)
            rig.binding.useTarget(target)
            assertEquals(1, rig.workers.size)
        }
    }

    @Test fun t943ThrowingCameraControlCannotSkipLocalStopOrNativeRetirement() {
        Rig().use { rig ->
            val authority = Authority().apply { failure = IllegalStateException("control failed") }
            rig.binding.start(); rig.binding.useTarget(rig.target(authority))
            val worker = rig.workers.single()
            rig.binding.useTarget(null)
            assertNull(rig.binding.selected)
            assertTrue(authority.claims.single().proofs.isEmpty())
            worker.release.complete(Unit)
            assertEquals(1, worker.retired)
            assertEquals(listOf(ResourceRetirement.RETIRED), authority.claims.single().proofs)
        }
    }

    @Test fun t943AdmissionFailureAfterRetainProvesNoStartWithoutLosingFailedOwner() {
        Rig().use { rig ->
            val authority = Authority()
            rig.binding.start(); rig.binding.useTarget(rig.target(authority))
            val original = rig.workers.single().apply { failure = IllegalStateException("native exit failed") }
            authority.retainHook = { original.release.complete(Unit) }
            rig.binding.choose(CameraLens.REAR)
            assertEquals(1, rig.workers.size)
            assertEquals(2, authority.claims.size)
            assertTrue(authority.claims.first().proofs.isEmpty())
            assertEquals(listOf(ResourceRetirement.NOT_STARTED), authority.claims.last().proofs)
            assertNull(rig.binding.selected)
            assertFalse(authority.available)
        }
    }

}
