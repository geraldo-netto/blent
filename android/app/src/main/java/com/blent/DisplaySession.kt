package com.blent

import kotlinx.coroutines.flow.StateFlow

/** Presentation contract. S is the platform boundary's borrowed display handle;
 * implementations own native execution and never destroy the caller's handle. */
internal interface DisplaySession<S> {
    var token: String?
    var mimeType: String
    var streamFps: Int
    var onFrameRendered: ((RenderAcknowledgement) -> Unit)?
    val startupStatus: StateFlow<String?>
    val retirementStatus: StateFlow<String?>
    val decoderDiagnostics: StateFlow<DecoderDiagnostics>
    fun observeConnection(connected: () -> Unit, disconnected: () -> Unit)
    fun setStreamFormat(format: DecoderFormat)
    fun setSurface(surfaceView: S)
    fun onSurfaceDestroyed()
    fun start()
    fun stop()
    fun getFps(): Float
    fun getMbps(): Float
}
