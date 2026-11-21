package com.blent

import android.hardware.camera2.CameraCharacteristics
import android.hardware.camera2.CaptureRequest
import android.hardware.camera2.params.MeteringRectangle

/** One capture worker owns this adapter; controls never escape its original device/session. */
internal class CameraTuning(private val metadata: CameraCharacteristics) {
    val capabilities = capabilities(metadata)

    fun apply(builder: CaptureRequest.Builder, values: CameraSensorTuning) {
        capabilities.validate(values)
        builder.set(CaptureRequest.CONTROL_AE_EXPOSURE_COMPENSATION,
            capabilities.exposureSteps(values.exposureHundredths))
        if (values.metering == 1) center(builder)
        noise(values.noise)?.let { builder.set(CaptureRequest.NOISE_REDUCTION_MODE, it) }
        flicker(values.flicker)?.let { builder.set(CaptureRequest.CONTROL_AE_ANTIBANDING_MODE, it) }
    }
    private fun center(builder: CaptureRequest.Builder) {
        val area = requireNotNull(metadata.get(CameraCharacteristics.SENSOR_INFO_ACTIVE_ARRAY_SIZE))
        val width = (area.width() / 2).coerceAtLeast(1)
        val height = (area.height() / 2).coerceAtLeast(1)
        val region = MeteringRectangle(area.left + (area.width() - width) / 2,
            area.top + (area.height() - height) / 2, width, height, 1000)
        builder.set(CaptureRequest.CONTROL_AE_REGIONS, arrayOf(region))
    }
    companion object {
        private fun noise(value: Int): Int? = when (value) {
            1 -> CaptureRequest.NOISE_REDUCTION_MODE_OFF
            2 -> CaptureRequest.NOISE_REDUCTION_MODE_FAST
            else -> null
        }
        private fun flicker(value: Int): Int? = when (value) {
            1 -> CaptureRequest.CONTROL_AE_ANTIBANDING_MODE_AUTO
            2 -> CaptureRequest.CONTROL_AE_ANTIBANDING_MODE_50HZ
            3 -> CaptureRequest.CONTROL_AE_ANTIBANDING_MODE_60HZ
            else -> null
        }
        private fun mask(available: IntArray?, choices: List<Int?>): Int = choices.withIndex()
            .fold(0) { mask, (index, value) ->
                if (value == null || available?.contains(value) == true) mask or (1 shl index) else mask
            }
        private fun capabilities(metadata: CameraCharacteristics): CameraTuningCapabilities {
            val range = metadata.get(CameraCharacteristics.CONTROL_AE_COMPENSATION_RANGE)
            val step = metadata.get(CameraCharacteristics.CONTROL_AE_COMPENSATION_STEP)
            val center = (metadata.get(CameraCharacteristics.CONTROL_MAX_REGIONS_AE) ?: 0) > 0 &&
                metadata.get(CameraCharacteristics.SENSOR_INFO_ACTIVE_ARRAY_SIZE) != null
            return CameraTuningCapabilities(range?.lower ?: 0, range?.upper ?: 0,
                step?.numerator ?: 0, step?.denominator ?: 1, center,
                mask(metadata.get(CameraCharacteristics.NOISE_REDUCTION_AVAILABLE_NOISE_REDUCTION_MODES), (0..2).map(::noise)),
                mask(metadata.get(CameraCharacteristics.CONTROL_AE_AVAILABLE_ANTIBANDING_MODES), (0..3).map(::flicker)))
        }
    }
}
