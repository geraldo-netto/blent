package com.blent

import android.hardware.camera2.CameraCharacteristics
import android.hardware.camera2.CaptureRequest
import android.graphics.Rect
import android.util.Range
import android.util.Rational
import okio.Buffer
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.Shadows
import org.robolectric.annotation.Config
import org.robolectric.shadows.ShadowCameraCharacteristics

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [27, 34])
class CameraTuningTest {
    private fun metadata(): CameraCharacteristics {
        val value = ShadowCameraCharacteristics.newCameraCharacteristics()
        val shadow = Shadows.shadowOf(value)
        shadow.set(CameraCharacteristics.CONTROL_AE_COMPENSATION_RANGE, Range(-6,6))
        shadow.set(CameraCharacteristics.CONTROL_AE_COMPENSATION_STEP, Rational(1,3))
        shadow.set(CameraCharacteristics.CONTROL_MAX_REGIONS_AE, 1)
        shadow.set(CameraCharacteristics.SENSOR_INFO_ACTIVE_ARRAY_SIZE, Rect(10,20,1010,820))
        shadow.set(CameraCharacteristics.NOISE_REDUCTION_AVAILABLE_NOISE_REDUCTION_MODES, intArrayOf(0,1))
        shadow.set(CameraCharacteristics.CONTROL_AE_AVAILABLE_ANTIBANDING_MODES, intArrayOf(1,2,3))
        return value
    }
    @Test fun t1468_advertisedControlsReachCaptureRequestAndResetThroughFreshTemplate() {
        val tuning = CameraTuning(metadata())
        for ((noise, flicker) in listOf(0 to 0, 1 to 1, 2 to 2, 0 to 3)) {
            val builder = CameraDeviceCaptureShadow().createCaptureRequest(3)
            tuning.apply(builder, CameraSensorTuning(100,1,noise,flicker))
            assertEquals(3, builder.get(CaptureRequest.CONTROL_AE_EXPOSURE_COMPENSATION))
            val region = requireNotNull(builder.get(CaptureRequest.CONTROL_AE_REGIONS)).single()
            assertEquals(Rect(260,220,760,620), region.rect)
        }
        val defaults = CameraDeviceCaptureShadow().createCaptureRequest(3)
        tuning.apply(defaults, CameraSensorTuning())
        assertNull(defaults.get(CaptureRequest.CONTROL_AE_REGIONS))
        assertNull(defaults.get(CaptureRequest.NOISE_REDUCTION_MODE))
    }
    @Test fun t1468_unsupportedAndOutOfRangeControlsRejectWithoutMutation() {
        val tuning = CameraTuning(ShadowCameraCharacteristics.newCameraCharacteristics())
        val builder = CameraDeviceCaptureShadow().createCaptureRequest(3)
        tuning.apply(builder, CameraSensorTuning())
        for (value in listOf(CameraSensorTuning(1), CameraSensorTuning(metering=1),
            CameraSensorTuning(noise=1), CameraSensorTuning(flicker=1))) {
            assertTrue(runCatching { tuning.apply(builder,value) }.isFailure)
        }
        val capabilities = CameraTuning(metadata()).capabilities
        for (ev in listOf(-301,-201,201,301)) {
            assertTrue(runCatching { capabilities.validate(CameraSensorTuning(ev)) }.isFailure)
        }
        assertEquals(1,capabilities.exposureSteps(33))
        assertEquals(-1,capabilities.exposureSteps(-33))
    }
    @Test fun t1468_boundedCapabilitiesAndControlWireHaveIndependentOracle() {
        val output = Buffer()
        CameraTuning(metadata()).capabilities.write(output)
        assertEquals(20, output.size.toInt())
        assertEquals(-6,output.readInt()); assertEquals(6,output.readInt())
        assertEquals(1,output.readInt()); assertEquals(3,output.readInt())
        assertEquals(1,output.readByte().toInt()); assertEquals(7,output.readByte().toInt())
        assertEquals(15,output.readByte().toInt()); assertEquals(0,output.readByte().toInt())
        val bytes = Buffer().writeLong(3).writeShort(100).writeByte(1).writeByte(2).writeByte(3)
            .writeShort(3000).writeShort(1000).writeByte(1).readByteArray()
        val expected = CameraTuningCommand(3,CameraSensorTuning(100,1,2,3),3000,1000,true)
        assertEquals(expected,CameraTuningCommand.read(Buffer().write(bytes)))
        for (size in bytes.indices) assertTrue(runCatching { CameraTuningCommand.read(Buffer().write(bytes,0,size)) }.isFailure)
        for (index in bytes.indices) {
            val changed=bytes.clone(); changed[index]=127
            runCatching { CameraTuningCommand.read(Buffer().write(changed)) }.onSuccess { it.validate() }
        }
    }
}
