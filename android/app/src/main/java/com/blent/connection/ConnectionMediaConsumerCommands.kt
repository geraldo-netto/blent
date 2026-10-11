package com.blent.connection

import com.blent.MediaLocalConsent

internal object ConnectionCameraConsumerCommands : ConnectionMediaConsumerCommands {
    override val resource = ConnectionManagement.Resource.CAMERA
    override fun stop(lease: ConnectionResourceLease) = ConnectionWorkerCommand.CameraChoice(lease, CameraConsentChoice(false, false, enabled = false))
    override fun retired(lease: ConnectionResourceLease) = ConnectionWorkerCommand.CameraConsumerRetired(lease)
}
internal class ConnectionAudioConsumerCommands(override val resource: ConnectionManagement.Resource) : ConnectionMediaConsumerCommands {
    init { require(resource == ConnectionManagement.Resource.MICROPHONE || resource == ConnectionManagement.Resource.SPEAKERS) }
    override fun stop(lease: ConnectionResourceLease): ConnectionWorkerCommand {
        ConnectionLocalLease.validate(lease, resource)
        return ConnectionWorkerCommand.AudioChoice(lease, MediaLocalConsent(false, false, false, false))
    }
    override fun retired(lease: ConnectionResourceLease): ConnectionWorkerCommand {
        ConnectionLocalLease.validate(lease, resource)
        return ConnectionWorkerCommand.AudioConsumerRetired(lease)
    }
}
