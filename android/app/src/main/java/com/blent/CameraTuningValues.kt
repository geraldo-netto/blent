package com.blent

import okio.BufferedSink
import okio.BufferedSource

/** Independent bounded camera controls. No Android objects or native resource ownership. */
internal data class CameraSensorTuning(
    val exposureHundredths: Int = 0,
    val metering: Int = 0, // device / center
    val noise: Int = 0, // device / off / fast
    val flicker: Int = 0, // device / auto / 50 Hz / 60 Hz
) {
    fun validate() {
        require(exposureHundredths in -300..300)
        require(metering in 0..1 && noise in 0..2 && flicker in 0..3)
    }
}
internal data class CameraTuningCommand(
    val revision: Long, val sensor: CameraSensorTuning,
    val bitrate: Int, val floor: Int, val adaptive: Boolean,
) {
    fun validate() {
        require(revision > 0)
        sensor.validate()
        require(bitrate in 256..20000 && floor in 256..20000)
    }
    companion object {
        fun read(input: BufferedSource): CameraTuningCommand = try {
            decode(input)
        } catch (error: IllegalArgumentException) {
            throw java.io.IOException("Invalid camera tuning command", error)
        }
        private fun decode(input: BufferedSource): CameraTuningCommand {
            val revision = input.readLong()
            val sensor = CameraSensorTuning(input.readShort().toInt(), input.readByte().toInt(),
                input.readByte().toInt(), input.readByte().toInt())
            val bitrate = input.readShort().toInt() and 65535
            val floor = input.readShort().toInt() and 65535
            val adaptive = input.readByte().toInt()
            require(adaptive in 0..1)
            return CameraTuningCommand(revision, sensor, bitrate, floor, adaptive == 1).also { it.validate() }
        }
    }
}
internal data class CameraTuningCapabilities(
    val minimum: Int, val maximum: Int, val numerator: Int, val denominator: Int,
    val center: Boolean, val noiseMask: Int, val flickerMask: Int,
) {
    fun exposureSteps(value: Int): Int {
        if (value == 0) return 0
        require(numerator > 0 && denominator > 0) { "Exposure compensation unavailable" }
        val requested = value.toDouble() * denominator / (100.0 * numerator)
        require(requested >= minimum - 0.001 && requested <= maximum + 0.001) { "Exposure outside camera range" }
        return kotlin.math.round(requested).toInt().coerceIn(minimum, maximum)
    }
    fun validate(value: CameraSensorTuning) {
        value.validate()
        exposureSteps(value.exposureHundredths)
        require(value.metering == 0 || center) { "Center metering unavailable" }
        require(noiseMask and (1 shl value.noise) != 0) { "Noise mode unavailable" }
        require(flickerMask and (1 shl value.flicker) != 0) { "Flicker mode unavailable" }
    }
    fun write(output: BufferedSink) {
        output.writeInt(minimum).writeInt(maximum).writeInt(numerator).writeInt(denominator)
        output.writeByte(if (center) 1 else 0).writeByte(noiseMask).writeByte(flickerMask).writeByte(0)
    }
}
