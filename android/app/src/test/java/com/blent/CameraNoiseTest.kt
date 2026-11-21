package com.blent

import android.hardware.camera2.CameraCharacteristics
import android.hardware.camera2.CaptureRequest
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.Shadows
import org.robolectric.annotation.Config
import org.robolectric.shadows.ShadowCameraCharacteristics

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [27, 34])
class CameraNoiseTest {
    @Test fun t1469_noiseModeUsesAdvertisedNativeValueAndFreshDefault() {
        val metadata=ShadowCameraCharacteristics.newCameraCharacteristics()
        Shadows.shadowOf(metadata).set(CameraCharacteristics.NOISE_REDUCTION_AVAILABLE_NOISE_REDUCTION_MODES,intArrayOf(0,1))
        val tuning=CameraTuning(metadata)
        assertEquals(7,tuning.capabilities.noiseMask)
        for ((selection, expected) in listOf(0 to null, 1 to 0, 2 to 1)) {
            val request=CameraDeviceCaptureShadow().createCaptureRequest(3)
            tuning.apply(request,CameraSensorTuning(noise=selection))
            assertEquals(expected,request.get(CaptureRequest.NOISE_REDUCTION_MODE))
        }
    }
    @Test fun t1469_unavailableNoiseModeRejectsBeforeRequestMutation() {
        val tuning=CameraTuning(ShadowCameraCharacteristics.newCameraCharacteristics())
        val request=CameraDeviceCaptureShadow().createCaptureRequest(3)
        for (noise in listOf(-1,1,2,3,Int.MAX_VALUE)) {
            assertThrows(IllegalArgumentException::class.java) { tuning.apply(request,CameraSensorTuning(noise=noise)) }
            assertNull(request.get(CaptureRequest.NOISE_REDUCTION_MODE))
            assertNull(request.get(CaptureRequest.CONTROL_AE_EXPOSURE_COMPENSATION))
        }
    }
}
