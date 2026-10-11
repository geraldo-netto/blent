package com.blent.connection

import java.io.IOException
import org.junit.Assert.*
import org.junit.Test

class TabletConnectionRunTest {
    @Test fun t932DefaultCompositionRetainsCameraOfferWithoutOpeningEndpointBeforePermission() {
        val setup = RuntimeSetup()
        val io = RunnerIo()
        val run = tabletConnectionRun(ConnectionWorkerSpec("fixture"), ConnectionSetupReady(io, setup.approval, 4096))
        val session = run.endpoints() as TabletSession
        val grant = session.nextFrame(0)!!
        assertEquals(ConnectionWire.CREDIT, grant.header.kind)
        session.written(grant, 0)
        session.receive(ConnectionFrame(ConnectionWire.Header(ConnectionWire.CREDIT, ConnectionWire.MANAGEMENT,
            1, 1, 0, 0, 4096), byteArrayOf(), 0, 0), 0)
        val invitation = ConnectionManagement.Offer(cameraLease().key,
            ConnectionManagement.Credential(ByteArray(32)), cameraProfile())
        val bytes = ConnectionManagement.encode(invitation)
        session.receive(ConnectionFrame(ConnectionWire.Header(ConnectionWire.DATA, ConnectionWire.MANAGEMENT,
            1, 1, 1, bytes.size, 0), bytes, 0, bytes.size), 0)
        run.advance(0)
        val camera = run as LocalCameraControl
        val offer = camera.offered()!!
        assertEquals(cameraProfile(), offer.profile)
        assertNull(camera.camera())
        assertTrue(camera.choose(offer.lease, CameraConsentChoice(true, false), 0))
        assertTrue(camera.offered()!!.permissionRequired)
        assertNotNull(camera.pending())
        assertNull(camera.camera())
        run.stop(0)
        io.requests.forEach { io.complete(it, 0, IOException("fixture operation cancelled")) }
        io.gone = true
        run.advance(1)
        assertNull(camera.offered())
        assertTrue(run.retired())
    }
    @Test fun t978ProductionCompositionAdvertisesOnlyDisplayCameraAndSpeakersAndKeepsAudioPending() {
        val setup = RuntimeSetup(); val io = RunnerIo()
        val run = tabletConnectionRun(ConnectionWorkerSpec("fixture"), ConnectionSetupReady(io, setup.approval, 4096))
        val session = run.endpoints() as TabletSession
        val credit = checkNotNull(session.nextFrame(0)); session.written(credit, 0)
        session.receive(ConnectionFrame(ConnectionWire.Header(ConnectionWire.CREDIT, ConnectionWire.MANAGEMENT,
            1, 1, 0, 0, 4096), byteArrayOf(), 0, 0), 0)
        val capabilities = checkNotNull(session.nextFrame(0))
        assertEquals(ConnectionManagement.Capabilities(11), ConnectionManagement.decode(capabilities.bytes.copyOfRange(capabilities.offset, capabilities.offset + capabilities.length)))
        session.written(capabilities, 0)
        val bytes = ConnectionManagement.encode(ConnectionManagement.Offer(audioLease().key,
            ConnectionManagement.Credential(ByteArray(32)), ConnectionManagement.Audio(1, 40, false)))
        session.receive(ConnectionFrame(ConnectionWire.Header(ConnectionWire.DATA, ConnectionWire.MANAGEMENT,
            1, 1, 1, bytes.size, 0), bytes, 0, bytes.size), 0)
        run.advance(0)
        val audio = run as LocalAudioControl
        val offer = checkNotNull(audio.audioOffered(ConnectionManagement.Resource.SPEAKERS))
        assertEquals(ConnectionManagement.Audio(1, 40, false), offer.profile)
        assertNull(audio.audio(ConnectionManagement.Resource.SPEAKERS))
        assertNull(audio.audioOffered(ConnectionManagement.Resource.MICROPHONE))
        assertFalse(audio.chooseAudio(audioLease(ConnectionManagement.Resource.MICROPHONE), com.blent.MediaLocalConsent(true, true, true, false), 0))
        run.stop(0); io.requests.forEach { io.complete(it, 0, IOException("fixture cancelled")) }; io.gone = true
        run.advance(1); assertTrue(run.retired()); assertNull(audio.audioOffered(ConnectionManagement.Resource.SPEAKERS))
    }

}
