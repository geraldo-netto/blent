package com.blent

import android.hardware.camera2.CameraDevice
import kotlinx.coroutines.CancellableContinuation

/** Device onClosed fences all its sessions before surfaces/codecs are released. */
internal class CameraDeviceStartup(resources: CameraResources, continuation: CancellableContinuation<CameraDevice>) : CameraDevice.StateCallback() {
    private val closure = CameraClosure<CameraDevice>(CameraDevice::close)
    private val startup = CameraStartup(resources, continuation, closure::close)

    override fun onOpened(camera: CameraDevice) {
        closure.bind(camera)
        startup.opened(camera)
    }

    override fun onDisconnected(camera: CameraDevice) = failed(camera, "Camera disconnected")
    override fun onError(camera: CameraDevice, error: Int) = failed(camera, "Camera error $error")
    override fun onClosed(camera: CameraDevice) = closure.completed(camera)

    private fun failed(camera: CameraDevice, message: String) {
        closure.bind(camera)
        startup.failed(camera, message)
    }

    fun abort(error: Exception) = startup.abort(error)
}
