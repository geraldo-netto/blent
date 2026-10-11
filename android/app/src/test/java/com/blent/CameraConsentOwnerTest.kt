package com.blent

import android.Manifest
import android.os.Looper
import com.blent.connection.*
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.RuntimeEnvironment
import org.robolectric.Shadows.shadowOf
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [27, 34])
class CameraConsentOwnerTest {
    @Test fun t944ApplicationObservesOriginalOffersWithoutActivityAndResamplesPermission() {
        WorkerTestBridge.install()
        CameraOwner.reset()
        val app = RuntimeEnvironment.getApplication()
        shadowOf(app).denyPermissions(Manifest.permission.CAMERA)
        val fixture = ConnectionUiFixture()
        try {
            val binding = CameraOwner.get(app)
            val controls = checkNotNull(CameraOwner.access)
            fixture.connect(); fixture.prompt()
            fixture.workers.last().report(ConnectionWorkerEvent.State(ConnectionWorkerPhase.AUTHENTICATED))
            fixture.workers.last().report(ConnectionWorkerEvent.CameraOffered(LocalCameraOffer(cameraLease(), cameraProfile(), false)))
            shadowOf(Looper.getMainLooper()).idle()
            val original = controls.state.value.offer!!
            assertFalse(controls.state.value.foreground)
            assertFalse(controls.start(original))
            assertTrue(fixture.workers.last().commands.isEmpty())
            val prompts = mutableListOf<CameraPermissionPrompt>()
            val attachment = CameraOwner.attach { prompts.add(it); true }
            assertTrue(controls.start(original))
            assertEquals(ConnectionWorkerCommand.CameraChoice(cameraLease(), CameraConsentChoice(true, false, true, false)),
                fixture.workers.last().commands.last())
            assertNull(binding.selected)
            fixture.workers.last().report(ConnectionWorkerEvent.CameraOffered(LocalCameraOffer(cameraLease(), cameraProfile(), true)))
            shadowOf(Looper.getMainLooper()).idle()
            val permission = controls.state.value.offer!!
            assertSame(original.owner, permission.owner)
            assertTrue(controls.state.value.enabled)
            assertEquals(1, prompts.size)
            // SDK reports grant, but the fresh Android permission still denies it.
            assertTrue(CameraOwner.permissionResult(prompts.single(), true))
            assertFalse(controls.state.value.enabled)
            assertEquals(ConnectionWorkerCommand.CameraPermission(cameraLease(), true,
                CameraConsentChoice(true, false, true, false)), fixture.workers.last().commands.last())
            assertNull(binding.selected)
            assertTrue(CameraOwner.detach(attachment))
            fixture.binding.disconnect()
            shadowOf(Looper.getMainLooper()).idle()
            assertNull(controls.state.value.offer)
            assertFalse(controls.start(original))
            CameraOwner.reset()
            assertNull(CameraOwner.access)
            assertFalse(CameraOwner.permissionResult(prompts.single(), true))
        } finally { CameraOwner.reset(); fixture.close() }
    }

    @Test fun t944ResetCancelsQueuedPublicationAndDoesNotRetainPreviousPolicy() {
        WorkerTestBridge.install()
        CameraOwner.reset()
        val fixture = ConnectionUiFixture()
        try {
            val app = RuntimeEnvironment.getApplication()
            CameraOwner.get(app)
            val previous = CameraOwner.access!!
            fixture.connect(); fixture.prompt()
            fixture.workers.last().report(ConnectionWorkerEvent.State(ConnectionWorkerPhase.AUTHENTICATED))
            fixture.workers.last().report(ConnectionWorkerEvent.CameraOffered(LocalCameraOffer(cameraLease(), cameraProfile(), false)))
            CameraOwner.reset()
            shadowOf(Looper.getMainLooper()).idle()
            assertNull(previous.state.value.offer)
            CameraOwner.get(app)
            val latest = CameraOwner.access!!
            assertNotSame(previous, latest)
            shadowOf(Looper.getMainLooper()).idle()
            assertNotNull(latest.state.value.offer)
            assertNull(previous.state.value.offer)
            assertFalse(latest.state.value.enabled)
            assertTrue(fixture.workers.last().commands.isEmpty())
        } finally { CameraOwner.reset(); fixture.close() }
    }
}
