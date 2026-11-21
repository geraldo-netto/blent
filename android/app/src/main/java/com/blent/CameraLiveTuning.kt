package com.blent

/** The original capture worker owns this bounded command/receipt state. Native
 * callbacks execute synchronously on that worker, before its resource retirement. */
internal class CameraLiveTuning(
    private val capabilities: CameraTuningCapabilities,
    private val sensor: (CameraSensorTuning) -> Unit,
    private val encoding: (CameraTuningCommand) -> Unit,
) {
    var revision = 0L; private set
    var status = 0; private set // accepted / unsupported / native failure
    private var accepted: CameraTuningCommand? = null

    fun accept(command: CameraTuningCommand) {
        if (command.revision <= revision) return
        status = try {
            command.validate()
            capabilities.validate(command.sensor)
            apply(command)
            accepted = command
            0
        } catch (_: IllegalArgumentException) { 1 }
        catch (_: Exception) { 2 }
        revision = command.revision
        android.util.Log.i("BlentCamera", "adjustment=$revision status=$status")
    }
    private fun apply(command: CameraTuningCommand) {
        val previous = accepted
        // Native calls can take effect before throwing. Never deduplicate a
        // later retry/revert against a command whose native state is uncertain.
        accepted = null
        if (previous?.sensor != command.sensor) sensor(command.sensor)
        if (!sameEncoding(previous, command)) encoding(command)
    }
    private fun sameEncoding(previous: CameraTuningCommand?, command: CameraTuningCommand): Boolean {
        if (previous == null) return false
        return previous.bitrate == command.bitrate && previous.floor == command.floor && previous.adaptive == command.adaptive
    }
}
