package com.blent

import android.Manifest
import android.content.ComponentName
import android.content.Intent
import android.view.WindowManager
import com.blent.connection.*
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.cancel
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.Robolectric
import org.robolectric.RuntimeEnvironment
import org.robolectric.Shadows
import org.robolectric.annotation.Config
import org.robolectric.util.ReflectionHelpers

@RunWith(ComposeFixtureRunner::class)
@Config(sdk = [27, 34])
class MediaWindowWakeTest {
    private fun awake(activity: MainActivity): Boolean =
        activity.window.attributes.flags and WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON != 0

    @Test fun t1235CameraOnlyHoldsActualWindowAndStopReleasesWithoutNativeRetirement() = runBlocking {
        WorkerTestBridge.install(); CameraOwner.reset(); AudioOwner.reset()
        val app = RuntimeEnvironment.getApplication()
        Shadows.shadowOf(app).grantPermissions(Manifest.permission.CAMERA)
        val native = CameraUsbCaptureOwnerTest.NativeEndpoint()
        Shadows.shadowOf(app).setComponentNameAndServiceForBindServiceForIntent(
            Intent(app, CameraWorkerService::class.java), ComponentName(app, CameraWorkerService::class.java), native)
        ConnectionUiFixture().use { fixture ->
            val rig = DisplayWakeIntegrationTest.Rig()
            val power = PowerRuntimeFixture()
            val controller = Robolectric.buildActivity(MainActivity::class.java).create()
            val activity = controller.get()
            activity.session.stop()
            ReflectionHelpers.getField<StreamingPowerBinding>(activity, "powerBinding").stop()
            ReflectionHelpers.setField(activity, "session", rig.session)
            ReflectionHelpers.setField(activity, "powerBinding", StreamingPowerBinding(power.app, power.runtime))
            var owner: DisplayPowerOwner? = null
            try {
                controller.start(); rig.opened()
                owner = checkNotNull(PowerServiceNotice.read(power.app.starts.single())).notice.original.owner
                rig.signal(false); rig.until { !awake(activity) }
                fixture.connect(); fixture.prompt()
                val worker = fixture.workers.last()
                worker.report(ConnectionWorkerEvent.State(ConnectionWorkerPhase.AUTHENTICATED))
                worker.report(ConnectionWorkerEvent.CameraOffered(LocalCameraOffer(cameraLease(), cameraProfile(), false)))
                val controls = checkNotNull(CameraOwner.access)
                rig.until { controls.state.value.offer != null }
                assertFalse("T1235 an offer is not active sharing", awake(activity))
                assertTrue(controls.start(checkNotNull(controls.state.value.offer)))
                worker.report(ConnectionWorkerEvent.Camera(cameraRoute()))
                rig.until { activity.cameras.selected == CameraLens.REAR && native.tasks.isNotEmpty() }
                rig.drain()
                assertTrue("T1235 foreground camera-only capture must keep its Window awake", awake(activity))
                assertTrue(controls.stop(checkNotNull(controls.state.value.offer)))
                worker.report(ConnectionWorkerEvent.CameraClosed(cameraLease()))
                rig.until { activity.cameras.selected == null }
                assertFalse("T1235 camera Stop must release its screen demand", awake(activity))
                assertFalse(worker.commands.any { it is ConnectionWorkerCommand.CameraConsumerRetired })
                controller.stop(); rig.drain()
                assertFalse(awake(activity))
            } finally {
                native.die(); controller.destroy(); CameraOwner.reset(); AudioOwner.reset(); rig.close()
                val original = owner
                if (original != null) power.finish(original) else power.scope.cancel()
            }
        }
    }
}
