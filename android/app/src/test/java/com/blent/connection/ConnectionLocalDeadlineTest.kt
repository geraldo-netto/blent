package com.blent.connection

import java.util.concurrent.TimeUnit
import org.junit.Assert.*
import org.junit.Test

internal class DeadlineEndpoint(val original: PromptEndpoint, var budget: ConnectionDeadline) :
    LocalEndpoint by original, ConnectionDeadlineSource {
    override fun nextDeadline(): ConnectionDeadline = if (original.retired()) ConnectionDeadline.IDLE else budget
}

class ConnectionLocalDeadlineTest {
    @Test fun t1095NativeBindIdleAcceptAdmissionAndRetirementHaveIndependentBudgets() {
        val native = LocalNativeFixture()
        val owner = LocalEndpointOwner(native, 0, ConnectionRunLimits(10, 5))
        try {
            assertTrue(native.bindEntered.await(3, TimeUnit.SECONDS))
            assertEquals(ConnectionDeadline.at(10), owner.nextDeadline())
            native.bindRelease.countDown(); setupAwait { owner.port() != null }
            owner.advance(100)
            assertEquals(ConnectionDeadline.IDLE, owner.nextDeadline())
            owner.admit(100)
            assertEquals(ConnectionDeadline.at(110), owner.nextDeadline())
            native.acceptRelease.countDown()
            setupAwait { owner.takeIo() != null }
            assertEquals(ConnectionDeadline.IDLE, owner.nextDeadline())
            owner.stop(100)
            assertEquals(ConnectionDeadline.at(105), owner.nextDeadline())
            owner.advance(105)
            assertEquals(ConnectionDeadline.IDLE, owner.nextDeadline())
            assertTrue(owner.quarantined()); assertFalse(owner.retired())
        } finally {
            owner.stop(105); native.release(); setupAwait { owner.retired() }
        }
        assertEquals(ConnectionDeadline.IDLE, owner.nextDeadline())
    }

    @Test fun t1095NativeCloseFailureHasNoNewBudgetAndCannotBecomeRetirementProof() {
        val native = LocalNativeFixture().apply { closeFailure = true; release() }
        val threads = ArrayList<Thread>(2)
        val owner = LocalEndpointOwner(native, 0, ConnectionRunLimits(10, 5), threads = { name, work ->
            check(threads.size < 2)
            Thread(work, name).apply { isDaemon = true; threads.add(this) }
        })
        try {
            owner.stop(0); setupAwait { owner.quarantined() }
            assertEquals(ConnectionDeadline.IDLE, owner.nextDeadline())
            assertFalse(owner.retired())
        } finally {
            owner.stop(0); native.release()
            threads.forEach { it.join(); assertFalse(it.isAlive) }
            native.closeFailure = false; native.close()
        }
        assertFalse("T1095 sticky native failure remains quarantined", owner.retired())
    }

    @Test fun t1095PumpCombinesProducerBackpressureAndBorrowedWriteFlushBudgets() {
        val rig = LocalPumpRig(); rig.endpoint.accepting = false
        assertEquals(ConnectionDeadline.IDLE, rig.pump.nextDeadline())
        rig.pump.advance(0)
        assertEquals(ConnectionDeadline.IDLE, rig.pump.nextDeadline())
        rig.read(byteArrayOf(4, 5), 1)
        assertEquals(ConnectionDeadline.at(11), rig.pump.nextDeadline())
        rig.incoming(); rig.pump.advance(2)
        assertEquals(ConnectionDeadline.at(11), rig.pump.nextDeadline())
        rig.endpoint.accepting = true; rig.pump.advance(3)
        assertEquals(ConnectionDeadline.at(12), rig.pump.nextDeadline())
        rig.io.complete(rig.io.write(), 3); rig.pump.advance(4)
        assertEquals(ConnectionDeadline.at(12), rig.pump.nextDeadline())
        rig.io.complete(rig.io.flush(), 0); rig.pump.advance(5)
        assertEquals(ConnectionDeadline.IDLE, rig.pump.nextDeadline())
        assertEquals(1, rig.endpoint.consumed.size)
        rig.pump.stop(6)
        assertEquals(ConnectionDeadline.at(11), rig.pump.nextDeadline())
        rig.pump.advance(11)
        assertEquals(ConnectionDeadline.IDLE, rig.pump.nextDeadline()); assertFalse(rig.pump.retired())
        rig.io.gone = true; rig.pump.advance(12)
        assertTrue(rig.pump.retired()); assertEquals(ConnectionDeadline.IDLE, rig.pump.nextDeadline())
    }

    @Test fun t1095KnownAndUnknownNativeGroupBudgetsRetainTheEarliestWitness() {
        val rig = ResourceRig()
        val first = DeadlineEndpoint(PromptEndpoint(25001), ConnectionDeadline.at(20))
        val second = DeadlineEndpoint(PromptEndpoint(25002), ConnectionDeadline.at(10))
        val endpoints = listOf(first, second)
        var selected = 0
        val group = LocalResourceGroup(ConnectionSessionEvent.Offered(rig.lease, rig.credential, ConnectionManagement.Display),
            rig.endpoint, 4096, ConnectionRunLimits(10, 5))
        assertEquals(ConnectionDeadline.IDLE, group.nextDeadline())
        group.open(0) { endpoints[selected++] }; group.advance(0)
        assertEquals(ConnectionDeadline.at(10), group.nextDeadline())
        second.budget = ConnectionDeadline.UNKNOWN
        assertEquals(ConnectionDeadline(20, false), group.nextDeadline())
        group.stop(1); endpoints.forEach { it.original.finish() }; group.advance(2)
        assertTrue(group.retired()); assertEquals(ConnectionDeadline.IDLE, group.nextDeadline())
    }

    @Test fun t1095UncertainFactoryNeverAdvertisesCompleteBudgetCoverage() {
        val rig = ResourceRig()
        val first = DeadlineEndpoint(PromptEndpoint(25001), ConnectionDeadline.at(20))
        var calls = 0
        val group = LocalResourceGroup(ConnectionSessionEvent.Offered(rig.lease, rig.credential, ConnectionManagement.Display),
            rig.endpoint, 4096, ConnectionRunLimits(10, 5))
        group.open(0) { if (calls++ == 0) first else error("T1095 uncertain partial factory") }
        assertEquals(ConnectionDeadline(20, false), group.nextDeadline())
        first.original.finish(); group.advance(1)
        assertFalse(group.retired()); assertTrue(group.quarantined())
        assertEquals(ConnectionDeadline.UNKNOWN, group.nextDeadline())
    }

    @Test fun t1095ConsumerBudgetsSurviveNativeGroupRetirementForCameraAndSpeaker() {
        val camera = CameraOwnerRig(); camera.start(); camera.admit()
        assertNotNull(camera.local.camera()); camera.close(2); camera.finish(3)
        assertEquals(ConnectionDeadline.at(7), camera.local.nextDeadline())
        assertFalse(camera.local.retired())
        camera.local.advance(7)
        assertEquals(ConnectionDeadline.IDLE, camera.local.nextDeadline()); assertTrue(camera.local.quarantined())
        assertTrue(camera.local.consumerRetired(camera.lease, 7))
        assertEquals(ConnectionDeadline.IDLE, camera.local.nextDeadline())
        val speaker = SpeakerRig(); speaker.start(); speaker.admit()
        assertNotNull(speaker.local.audio(ConnectionManagement.Resource.SPEAKERS)); speaker.close(2); speaker.finish(3)
        assertEquals(ConnectionDeadline.at(7), speaker.local.nextDeadline())
        assertFalse(speaker.local.retired())
        assertTrue(speaker.local.audioConsumerRetired(speaker.lease, 3))
        assertEquals(ConnectionDeadline.IDLE, speaker.local.nextDeadline())
    }

    @Test fun t1095PermissionWaitExposesNoNativeBudgetOrEndpointForAnyMediaDirection() {
        val rig = TabletSessionRig(timeout = 100)
        var opened = 0
        val local = ConnectionLocalResources(rig.tablet, 4096, factory = { opened++; error("T1095 no consent") },
            cameraAuthority = rig.tablet, audioAuthority = rig.tablet, microphoneAuthority = rig.tablet)
        for (resource in listOf(ConnectionManagement.Resource.CAMERA, ConnectionManagement.Resource.SPEAKERS,
                ConnectionManagement.Resource.MICROPHONE)) {
            rig.send(rig.invitation(resource)); local.advance(0)
            assertEquals(ConnectionDeadline.IDLE, local.nextDeadline())
            assertEquals(ConnectionDeadline.at(100), rig.tablet.nextDeadline())
        }
        local.stop(0); assertTrue(local.retired()); assertEquals(0, opened)
        assertEquals(ConnectionDeadline.IDLE, local.nextDeadline())
    }

    @Test fun t1095UncertainRetirementPublicationRemainsAnUnknownBudgetProvider() {
        val rig = CameraOwnerRig()
        val endpoint = object : ConnectionEndpoints by rig.endpoint {
            override fun nativeRetired(lease: ConnectionResourceLease) { error("T1095 uncertain retirement publication") }
        }
        val permit = object : LocalResourcePermit {
            override fun opening(lease: ConnectionResourceLease): Boolean = rig.owns(lease, 0)
            override fun admission(lease: ConnectionResourceLease): Boolean = rig.owns(lease, 0)
        }
        val native = LocalMediaResource(ConnectionSessionEvent.Offered(rig.lease, rig.credential, rig.profile),
            endpoint, 4096, ConnectionRunLimits(10, 5), rig, permit, {})
        val channel = PromptEndpoint(25001)
        native.open(0) { channel }; native.prepare(0); native.advance(0)
        native.stop(0); channel.finish(); native.advance(1)
        assertEquals(ConnectionDeadline.UNKNOWN, native.nextDeadline())
        assertFalse(native.retired()); assertTrue(native.quarantined())
    }
}
