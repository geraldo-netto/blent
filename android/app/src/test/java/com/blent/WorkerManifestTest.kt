package com.blent

import android.content.ComponentName
import android.content.pm.PackageManager
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.RuntimeEnvironment
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [27, 34])
class WorkerManifestTest {
    @Test fun t809_nativeOwnersHavePrivateIndependentProcesses() {
        val app = RuntimeEnvironment.getApplication()
        val processes = listOf("DisplayWorkerService", "CameraWorkerService", "MicrophoneWorkerService", "SpeakerWorkerService", "MetadataWorkerService", "PowerWorkerService").map { name ->
            val info = app.packageManager.getServiceInfo(ComponentName(app.packageName, "com.blent.$name"), PackageManager.GET_META_DATA)
            assertFalse("T809 worker exported", info.exported)
            assertNotEquals("T809 native worker shares UI process", app.applicationInfo.processName, info.processName)
            info.processName
        }
        assertEquals("T809 native resources share a failure domain", processes.size, processes.toSet().size)
    }
}
