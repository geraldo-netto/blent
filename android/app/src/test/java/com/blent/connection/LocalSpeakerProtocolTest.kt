package com.blent.connection

import com.blent.MediaLocalConsent
import org.junit.Assert.*
import org.junit.Test
import java.io.ByteArrayOutputStream

class LocalSpeakerProtocolTest {
    private val speakers = ConnectionManagement.Resource.SPEAKERS
    @Test fun t984RealManagementAdmissionPumpsFragmentedBootstrapAndClockedPcmWithoutBinder() {
        val rig = TabletSessionRig(quantum = 7)
        val natives = ArrayList<PromptEndpoint>()
        val local = ConnectionLocalResources(rig.tablet, 4096, factory = {
            PromptEndpoint(24000).also { natives.add(it) }
        }, audioAuthority = rig.tablet)
        rig.send(rig.invitation(speakers)); local.advance(0)
        val offer = checkNotNull(local.audioOffered(speakers)); assertTrue(natives.isEmpty())
        assertTrue(local.chooseAudio(offer.lease, MediaLocalConsent(true, true, true, false), 0))
        local.advance(0); rig.pump()
        assertTrue(rig.messages.contains(ConnectionManagement.Prepared(offer.lease.key)))
        rig.send(ConnectionManagement.Commit(offer.lease.key)); rig.host.activate(ConnectionWire.SPEAKER, 1); rig.pump(); local.advance(0)
        val route = checkNotNull(local.audio(speakers)); assertEquals(offer.profile, route.profile)
        val io = natives.single().io
        val bootstrap = "BLAUREQ2".toByteArray() + ByteArray(64) { 'a'.code.toByte() } + byteArrayOf(2, 1, 0, 40)
        produce(io, bootstrap, local, rig)
        assertArrayEquals(bootstrap, rig.payloads.filter { it.first == ConnectionWire.SPEAKER }.flatMap { it.second.asIterable() }.toByteArray())
        val replyAndPcm = ByteArray(92 + 1972) { (it * 31).toByte() }
        assertTrue(rig.host.offer(ConnectionWire.SPEAKER, replyAndPcm)); rig.pump(); local.advance(0)
        assertArrayEquals(replyAndPcm, consume(io, replyAndPcm.size, local, rig))
        rig.tablet.cancel(offer.lease, ConnectionManagement.Reason.REQUESTED); local.advance(0)
        assertNull(local.audio(speakers)); assertFalse(local.audioConsumerRetired(offer.lease.copy(owner = 2), 0))
        assertTrue(local.audioConsumerRetired(offer.lease, 0)); natives.single().finish(); local.advance(0)
        rig.peerClose(offer.lease); rig.send(ConnectionManagement.Revoke(offer.lease.key, ConnectionManagement.Reason.REQUESTED)); rig.pump(); local.advance(0)
        assertTrue(rig.messages.contains(ConnectionManagement.ResultMessage(offer.lease.key, ConnectionManagement.Outcome.REVOKED)))
        local.stop(0); assertTrue(local.retired())
    }
    private fun produce(io: RunnerIo, bytes: ByteArray, local: ConnectionLocalResources, rig: TabletSessionRig) {
        for (chunk in bytes.asList().chunked(7)) {
            val read = io.read(); chunk.forEachIndexed { index, value -> read.loan.bytes[index] = value }
            io.complete(read, chunk.size); local.advance(0); rig.pump()
        }
    }
    private fun consume(io: RunnerIo, count: Int, local: ConnectionLocalResources, rig: TabletSessionRig): ByteArray {
        val bytes = ByteArrayOutputStream(); var cursor = 0
        var lastFlush: ConnectionIoRequest.Flush? = null
        repeat(4096) {
            while (cursor < io.requests.size) {
                when (val request = io.requests[cursor++]) {
                    is ConnectionIoRequest.Read -> Unit
                    is ConnectionIoRequest.Write -> {
                        val size = minOf(7, request.loan.length)
                        bytes.write(request.loan.bytes, request.loan.offset, size); io.complete(request, size)
                    }
                    is ConnectionIoRequest.Flush -> { io.complete(request, 0); lastFlush = request }
                }
            }
            local.advance(0); rig.pump()
            if (bytes.size() == count && io.requests.lastOrNull() === lastFlush && io.results.isEmpty()) return bytes.toByteArray()
        }
        error("T984 bounded local PCM writes did not finish")
    }
    @Test fun t984JoinedControlForwardsOnlyCurrentSelectionAndAcceptsLateOriginalRetirement() {
        val endpoints = ResourceEndpoints()
        val display = LocalPumpRig().lease
        val lease = display.copy(key = display.key.copy(resource = speakers))
        val local = ConnectionLocalResources(endpoints, 4096, factory = { PromptEndpoint(24001).apply { finish() } },
            audioAuthority = object : ConnectionResourceAuthority { override fun owns(lease: ConnectionResourceLease, now: Long) = lease == display.copy(key = display.key.copy(resource = speakers)) })
        val run = ConnectionLocalRun(RuntimeRun(), local)
        assertNull(run.audioOffered(speakers)); assertNull(run.audio(speakers))
        assertFalse(run.chooseAudio(lease, MediaLocalConsent(true, true, true, false), 0))
        endpoints.events.add(ConnectionSessionEvent.Offered(lease, ConnectionManagement.Credential(ByteArray(32)), ConnectionManagement.Audio(1, 40, false)))
        run.advance(0); assertNotNull(run.audioOffered(speakers))
        assertTrue(run.chooseAudio(lease, MediaLocalConsent(true, true, true, false), 0)); run.advance(0)
        endpoints.state(lease, ConnectionResourceState.ADMITTED); run.advance(0); assertNotNull(run.audio(speakers))
        run.stop(0); assertNull(run.audioOffered(speakers)); assertNull(run.audio(speakers))
        assertFalse(run.chooseAudio(lease, MediaLocalConsent(true, true, true, false), 0))
        assertFalse(run.audioConsumerRetired(lease.copy(owner = 2), 0)); assertTrue(run.audioConsumerRetired(lease, 0))
        val unsupported = ConnectionLocalRun(RuntimeRun(), object : LocalConnectionResources {
            override fun advance(now: Long) {}
            override fun display(): LocalDisplayRoute? = null
            override fun stop(now: Long) {}
            override fun retired() = true
            override fun quarantined() = false
        })
        assertNull(unsupported.audioOffered(speakers)); assertNull(unsupported.audio(speakers))
        assertFalse(unsupported.chooseAudio(lease, MediaLocalConsent(true, true, true, false), 0))
        assertFalse(unsupported.audioConsumerRetired(lease, 0))
    }
}
