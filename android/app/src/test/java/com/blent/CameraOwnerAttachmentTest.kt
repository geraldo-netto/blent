package com.blent

import android.Manifest
import android.os.Looper
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.RuntimeEnvironment
import org.robolectric.Shadows.shadowOf
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [27, 34])
class CameraOwnerAttachmentTest {
    @Test fun t961ResetRejectsDetachedSdkResultAndRequiresInitializedOwner() {
        WorkerTestBridge.install()
        CameraOwner.reset()
        assertFalse(CameraOwner.detach(LocalPermissionAttachment()))
        try { CameraOwner.attach { true }; fail("T961 uninitialized owner admitted UI") }
        catch (_: IllegalStateException) {}
        val app = RuntimeEnvironment.getApplication()
        shadowOf(app).denyPermissions(Manifest.permission.CAMERA)
        val binding = CameraOwner.get(app)
        val published = mutableListOf<LocalPermissionPrompt>()
        val attachment = CameraOwner.attach { published.add(it); true }
        try {
            CameraInvitations.endpoint.value = CameraEndpoint("ab".repeat(32), 12345, 640, 480, 30, 1000, CameraLens.FRONT)
            shadowOf(Looper.getMainLooper()).idle()
            assertEquals(1, published.size)
            assertTrue(CameraOwner.permissionResult(published.single(), false))
            assertNull(binding.selected)
            assertFalse(CameraOwner.permissionResult(published.single(), true))
            binding.choose(CameraLens.REAR)
            shadowOf(Looper.getMainLooper()).idle()
            val pending = published.last()
            assertTrue(CameraOwner.detach(attachment))
            assertFalse(CameraOwner.permissionResult(pending, true))
            CameraOwner.reset()
            assertFalse(CameraOwner.detach(attachment))
            assertFalse(CameraOwner.permissionResult(pending, true))
            assertNull(binding.endpoint)
        } finally { CameraOwner.reset() }
    }
}
