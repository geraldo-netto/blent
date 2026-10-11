package com.blent.connection

/** Camera event adapter for the shared change-only original publication owner. */
internal class ConnectionCameraPublication(report: (ConnectionWorkerEvent) -> Unit) {
    private val publication = ConnectionMediaPublication<LocalCameraOffer, LocalCameraRoute>(
        { report(ConnectionWorkerEvent.CameraClosed(it)) },
        { report(ConnectionWorkerEvent.CameraOffered(it)) },
        { report(ConnectionWorkerEvent.Camera(it)) },
    )
    fun update(offer: LocalCameraOffer?, route: LocalCameraRoute?) = publication.update(offer, route)
}
