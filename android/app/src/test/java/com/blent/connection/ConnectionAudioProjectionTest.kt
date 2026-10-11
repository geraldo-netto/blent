package com.blent.connection

import com.blent.MediaLocalConsent
import org.junit.Assert.*
import org.junit.Test

class ConnectionAudioProjectionTest {
    private val resources = listOf(ConnectionManagement.Resource.MICROPHONE, ConnectionManagement.Resource.SPEAKERS)
    @Test fun t989PublicationClosesBeforeReplacementAndRetainsTransitionAcrossReporterFailure() {
        val events = mutableListOf<ConnectionWorkerEvent>(); val publication = ConnectionAudioPublication(events::add)
        val route = audioRoute(); val offer = LocalAudioOffer(route.lease, route.profile)
        publication.update(offer, null); publication.update(offer, null)
        assertEquals(1, events.size); publication.update(null, route)
        assertEquals(2, events.size); assertTrue(events.none { it is ConnectionWorkerEvent.AudioClosed })
        val next = route.copy(lease = audioLease(owner = 2))
        publication.update(null, next)
        assertEquals(ConnectionWorkerEvent.AudioClosed(route.lease), events[2]); assertWorkerMetadata(ConnectionWorkerEvent.AudioRoute(next), events[3])
        publication.update(null, null); publication.update(null, null)
        assertEquals(ConnectionWorkerEvent.AudioClosed(next.lease), events.last())
        assertThrows(IllegalArgumentException::class.java) { publication.update(offer, route) }
        val failed = ConnectionAudioPublication { error("T989 publication failed") }
        assertThrows(IllegalStateException::class.java) { failed.update(offer, null) }
        failed.update(offer, null) // Retained transition is not re-emitted after failure.
    }
    @Test fun t989DirectionPresentationsRejectForeignProfileOwnerTicketAndReplayWithoutChangingOriginalChoice() {
        for (resource in resources) {
            val projection = ConnectionAudioPresentation(resource); val route = audioRoute(resource)
            val ticket = route.lease.connection; val offer = LocalAudioOffer(route.lease, route.profile)
            projection.offered(ticket, offer); val original = checkNotNull(projection.offer)
            projection.offered(ticket, offer); assertSame(original, projection.offer); assertTrue(projection.current(original))
            projection.offered(ticket.copy(epoch = 2), offer); assertSame(original, projection.offer)
            projection.offered(ticket, offer.copy(profile = offer.profile.copy(bufferMs = 50))); assertSame(original, projection.offer)
            projection.admitted(ticket, route.copy(profile = route.profile.copy(bufferMs = 50))); assertNull(projection.route)
            projection.admitted(ticket, route); val admitted = checkNotNull(projection.route)
            assertFalse(projection.current(original)); assertTrue(projection.current(admitted))
            projection.admitted(ticket, route); projection.offered(ticket, offer); assertSame(admitted, projection.route)
            val other = if (resource == resources.first()) resources.last() else resources.first()
            projection.closed(ticket, audioLease(other)); projection.offered(ticket, LocalAudioOffer(audioLease(other), offer.profile))
            projection.admitted(ticket, audioRoute(other)); assertSame(admitted, projection.route)
            projection.closed(ticket.copy(epoch = 2), route.lease); assertSame(admitted, projection.route)
            projection.closed(ticket, route.lease); assertNull(projection.route)
            projection.offered(ticket, offer); assertNull(projection.offer)
            projection.reset(); projection.offered(ticket, offer); assertNotNull(projection.offer)
            projection.clear(); assertNull(projection.offer)
        }
    }
    @Test fun t989PortableReducerBindsLocalAudioChoicesToExactGenerationAndClearsOnLifecycleLoss() {
        val reducer = ConnectionPresentation(); val token = reducer.begin(); val ticket = audioLease().connection
        for (resource in resources) {
            val route = audioRoute(resource); val event = ConnectionWorkerEvent.AudioOffered(LocalAudioOffer(route.lease, route.profile))
            reducer.event(token, event); assertNull(reducer.state.value.speakersOffer); assertNull(reducer.state.value.microphoneOffer)
        }
        reducer.event(token, ConnectionWorkerEvent.Peer(ticket, ticket.peer, false))
        reducer.event(token, ConnectionWorkerEvent.State(ConnectionWorkerPhase.AUTHENTICATED))
        for (resource in resources) {
            val route = audioRoute(resource)
            reducer.event(token, ConnectionWorkerEvent.AudioOffered(LocalAudioOffer(route.lease, route.profile)))
            val choice = if (resource == resources.first()) checkNotNull(reducer.state.value.microphoneOffer) else checkNotNull(reducer.state.value.speakersOffer)
            val context = MediaLocalConsent(true, true, true, false)
            assertEquals(ConnectionWorkerCommand.AudioChoice(route.lease, context), reducer.audioChoice(token, choice, context))
            assertNull(reducer.audioChoice(Any(), choice, context))
            assertNull(reducer.audioChoice(token, ConnectionAudioOfferChoice(choice.offer), context))
            reducer.event(token, ConnectionWorkerEvent.AudioRoute(route))
            assertNull(reducer.audioChoice(token, choice, context))
        }
        reducer.stop(token); assertNull(reducer.state.value.microphone); assertNull(reducer.state.value.speakers)
        reducer.retired(token)
        reducer.event(token, ConnectionWorkerEvent.AudioRoute(audioRoute())); assertNull(reducer.state.value.speakers)
        val next = reducer.begin(); reducer.event(token, ConnectionWorkerEvent.AudioRoute(audioRoute()))
        assertNull(reducer.state.value.speakers); reducer.ended(next, "T989 source ended")
    }
}
