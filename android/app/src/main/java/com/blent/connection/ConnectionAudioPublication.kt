package com.blent.connection

/** Audio event adapter for the shared change-only original publication owner. */
internal class ConnectionAudioPublication(report: (ConnectionWorkerEvent) -> Unit) {
    private val publication = ConnectionMediaPublication<LocalAudioOffer, LocalAudioRoute>(
        { report(ConnectionWorkerEvent.AudioClosed(it)) },
        { report(ConnectionWorkerEvent.AudioOffered(it)) },
        { report(ConnectionWorkerEvent.AudioRoute(it)) },
    )
    fun update(offer: LocalAudioOffer?, route: LocalAudioRoute?) = publication.update(offer, route)
}
