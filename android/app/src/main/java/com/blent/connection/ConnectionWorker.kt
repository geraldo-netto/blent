package com.blent.connection

import com.blent.LifecycleRun
import com.blent.MediaLocalConsent

/** Local device selection only; never a certificate, permission or media grant. */
internal data class ConnectionWorkerSpec(
    val device: String, val session: ConnectionSessionLimits = ConnectionSessionLimits(),
    val io: ConnectionRunLimits = ConnectionRunLimits(), val approvalTimeoutMs: Long = 120000,
    val nativeTimeoutMs: Long = 10000, val trustedPeers: Int = 64,
    val accessoryCapacity: Int = 32, val timerMs: Long = 100,
) {
    init {
        require(device.isNotEmpty() && device.length <= 512 && '\u0000' !in device)
        require(approvalTimeoutMs in 1..600000 && nativeTimeoutMs in 1..60000)
        require(trustedPeers in 1..1024 && accessoryCapacity in 1..1024)
        require(timerMs in 1..1000)
    }
    fun setupLimits(): ConnectionSetupLimits = ConnectionSetupLimits(
        session.capacity, approvalTimeoutMs, nativeTimeoutMs, io.retirementTimeoutMs, trustedPeers,
    )
    fun waitLimits(): ConnectionWaitLimits {
        val native = minOf(nativeTimeoutMs, approvalTimeoutMs)
        val transport = minOf(io.ioTimeoutMs, io.retirementTimeoutMs, session.resourceTimeoutMs)
        return ConnectionWaitLimits(timerMs, minOf(native, transport))
    }
}
internal sealed interface ConnectionWorkerCommand {
    data class Decide(val ticket: ConnectionTicket, val approved: Boolean, val remember: Boolean) : ConnectionWorkerCommand
    data class Revoke(val peer: ConnectionPeer) : ConnectionWorkerCommand
    sealed interface Camera : ConnectionWorkerCommand { val lease: ConnectionResourceLease }
    /** Local context must be sampled again by the native adapter before capture. */
    data class CameraChoice(override val lease: ConnectionResourceLease, val choice: CameraConsentChoice) : Camera {
        init { ConnectionLocalLease.validate(lease, ConnectionManagement.Resource.CAMERA) }
    }
    /** The original worker resolves this to its retained opaque pending prompt. */
    data class CameraPermission(override val lease: ConnectionResourceLease, val granted: Boolean, val choice: CameraConsentChoice) : Camera {
        init { ConnectionLocalLease.validate(lease, ConnectionManagement.Resource.CAMERA) }
    }
    /** Native capture completion/no-start proof, never stop, timeout or socket EOF. */
    data class CameraConsumerRetired(override val lease: ConnectionResourceLease) : Camera {
        init { ConnectionLocalLease.validate(lease, ConnectionManagement.Resource.CAMERA) }
    }
    sealed interface Audio : ConnectionWorkerCommand { val lease: ConnectionResourceLease }
    data class AudioChoice(override val lease: ConnectionResourceLease, val choice: MediaLocalConsent) : Audio {
        init { ConnectionAudioLease.validate(lease) }
    }
    /** The retained original worker resolves only its pending microphone prompt. */
    data class AudioPermission(override val lease: ConnectionResourceLease, val granted: Boolean, val choice: MediaLocalConsent) : Audio {
        init { ConnectionLocalLease.validate(lease, ConnectionManagement.Resource.MICROPHONE) }
    }
    /** Actual audio consumer retirement/no-start proof to its original worker. */
    data class AudioConsumerRetired(override val lease: ConnectionResourceLease) : Audio {
        init { ConnectionAudioLease.validate(lease) }
    }

}
internal enum class ConnectionWorkerPhase { OPENING, WAITING_APPROVAL, AUTHENTICATED, CLOSING }
internal sealed interface ConnectionWorkerEvent {
    /** Claimed peer and stored trust are presentation metadata, never TLS proof. */
    data class Peer(val ticket: ConnectionTicket, val local: ConnectionPeer, val remembered: Boolean) : ConnectionWorkerEvent
    data class Pairing(val ticket: ConnectionTicket, val local: ConnectionPeer) : ConnectionWorkerEvent
    data class State(val phase: ConnectionWorkerPhase) : ConnectionWorkerEvent
    /** Emitted only after durable trust mutation, never an optimistic UI promise. */
    data class Trust(val peer: ConnectionPeer, val remembered: Boolean) : ConnectionWorkerEvent
    /** Only private same-UID metadata; encoded video never travels over Binder. */
    sealed interface Media : ConnectionWorkerEvent
    data class Display(val route: LocalDisplayRoute) : Media
    data class DisplayClosed(val lease: ConnectionResourceLease) : Media {
        init { ConnectionDisplayLease.validate(lease) }
    }
    /** Offer contains no credential or endpoint; permission remains separate. */
    data class CameraOffered(val offer: LocalCameraOffer) : Media
    data class Camera(val route: LocalCameraRoute) : Media
    data class CameraClosed(val lease: ConnectionResourceLease) : Media {
        init { ConnectionLocalLease.validate(lease, ConnectionManagement.Resource.CAMERA) }
    }
    /** Flat private control metadata only; PCM uses the independent channel. */
    sealed interface Audio : Media { val lease: ConnectionResourceLease }
    data class AudioOffered(val offer: LocalAudioOffer) : Audio { override val lease: ConnectionResourceLease get() = offer.lease }
    data class AudioRoute(val route: LocalAudioRoute) : Audio { override val lease: ConnectionResourceLease get() = route.lease }
    data class AudioClosed(override val lease: ConnectionResourceLease) : Audio {
        init { ConnectionAudioLease.validate(lease) }
    }
    /** Bounded saved-trust metadata; never a pairing or media authority event. */
    data class SavedComputers(val page: SavedComputersPage) : ConnectionWorkerEvent
    /** Execution ended; neither native retirement nor process death is implied. */
    data class Ended(val failure: String?) : ConnectionWorkerEvent
}
/** command only admits bounded control work; it never performs native I/O. */
internal interface ConnectionWorker : LifecycleRun {
    fun command(value: ConnectionWorkerCommand): Boolean
}
