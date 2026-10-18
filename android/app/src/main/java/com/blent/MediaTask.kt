package com.blent

/** Independent, immutable requests. Native handles travel only through OS adapters. */
internal sealed interface MediaTask {
    data class Audio(val endpoint: AudioEndpoint, val preferences: AudioPreferences) : MediaTask
    data class Camera(val endpoint: CameraEndpoint, val lens: CameraLens, val rotation: Int) : MediaTask
    data class Display(val routes: DisplayRoutes, val format: DecoderFormat, val liveness: Boolean = false) : MediaTask
    data class Metadata(val width: Int = 0, val height: Int = 0, val fps: Int = 0, val shellVersion: Int = 0) : MediaTask
    data class Wake(val cpu: Boolean, val wifi: Boolean) : MediaTask
}

internal sealed interface WorkerEvent {
    data class Status(val text: String) : WorkerEvent
    data class Result(val text: String) : WorkerEvent
    data class Ended(val failure: String?) : WorkerEvent
    data class Rendered(val sequence: Int, val decodeMicros: Int, val decoder: String?) : WorkerEvent
    data class Display(val connected: Boolean, val fps: Float, val mbps: Float,
        val startup: String?, val retirement: String?, val diagnostics: DecoderDiagnostics) : WorkerEvent
    data class WakeReady(val cpu: Boolean, val wifi: Boolean) : WorkerEvent
}
