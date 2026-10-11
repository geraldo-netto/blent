package com.blent

import kotlinx.coroutines.*
import kotlinx.coroutines.flow.MutableStateFlow
import org.junit.Assert.*
import org.junit.Test

class CameraPermissionIdentityTest {
    private val endpoint = CameraEndpoint("ab".repeat(32), 12345, 640, 480, 30, 1000, CameraLens.FRONT)
    private inner class Rig : AutoCloseable {
        val scope = CoroutineScope(SupervisorJob() + Dispatchers.Unconfined)
        val invitations = MutableStateFlow<CameraEndpoint?>(null)
        var allowed = false
        val lenses = mutableListOf<CameraLens>()
        val binding = CameraBinding({ 0 }, {}, { allowed }, { _, lens, _, _ ->
            object : LifecycleRun {
                override suspend fun execute() { lenses.add(lens); awaitCancellation() }
                override fun cancel() {}
                override suspend fun retire() {}
            }
        }, invitations, scope, {})
        init { binding.start(); invitations.value = endpoint }
        override fun close() { binding.shutdown(); scope.cancel() }
    }

    @Test fun t950OldGrantCannotApproveLatestLensOrConsumeItsRequest() = Rig().use { rig ->
        val first = rig.binding.permissionPending!!
        rig.binding.choose(CameraLens.REAR)
        val latest = rig.binding.permissionPending!!
        assertNotSame(first, latest)
        rig.allowed = true
        rig.binding.permissionResult(true, first)
        assertSame("T950 stale grant consumed replacement request", latest, rig.binding.permissionPending)
        assertTrue("T950 stale grant started replacement lens", rig.lenses.isEmpty())
        rig.binding.permissionResult(true, latest)
        assertEquals(listOf(CameraLens.REAR), rig.lenses)
        rig.binding.permissionResult(false, latest)
        assertEquals(CameraLens.REAR, rig.binding.selected)
    }

    @Test fun t950DuplicateMetadataDenialAndForgedResultsKeepOriginalPending() = Rig().use { rig ->
        val first = rig.binding.permissionPending!!
        rig.binding.choose(CameraLens.FRONT)
        val latest = rig.binding.permissionPending!!
        for (old in listOf(first, CameraCapturePermission(endpoint, CameraLens.FRONT), null)) {
            rig.binding.permissionResult(false, old)
            assertSame(latest, rig.binding.permissionPending)
        }
        rig.binding.permissionResult(false, latest)
        assertNull(rig.binding.permissionPending)
        assertEquals("Camera permission denied.", rig.binding.status)
        assertTrue(rig.lenses.isEmpty())
    }

    @Test fun t950StoppedReplacedAndGrantedWithoutFreshPermissionCannotCapture() = Rig().use { rig ->
        val first = rig.binding.permissionPending!!
        rig.binding.stop()
        rig.allowed = true
        rig.binding.permissionResult(true, first)
        assertTrue(rig.lenses.isEmpty())
        rig.allowed = false
        rig.binding.start()
        val request = rig.binding.permissionPending!!
        rig.binding.permissionResult(true, request)
        val repeated = rig.binding.permissionPending!!
        assertNotSame(request, repeated)
        assertTrue(rig.lenses.isEmpty())
        rig.invitations.value = endpoint.copy(port = 12346)
        val replacement = rig.binding.permissionPending!!
        rig.binding.permissionResult(true, repeated)
        assertSame(replacement, rig.binding.permissionPending)
        rig.allowed = true
        rig.binding.permissionResult(true, replacement)
        assertEquals(listOf(CameraLens.FRONT), rig.lenses)
    }
}
