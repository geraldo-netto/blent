package com.blent.connection

import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [27, 34])
class ConnectionJoinedCameraFixtureTest {
    @Test fun t1004HostOfferCannotMutateTabletBetweenActorTurns() {
        for (resource in ConnectionManagement.Resource.entries) {
            val rig = ConnectionJoinedCameraFixture(15)
            try {
                rig.send(rig.protocol.invitation(resource))
                assertNull("T1004 host input bypassed actor advancement", rig.protocol.tablet.nextEvent())
                synchronized(rig.gate) { rig.protocol.pump() }
                val offered = rig.protocol.tablet.nextEvent() as ConnectionSessionEvent.Offered
                assertEquals(resource, offered.lease.key.resource)
                assertEquals(0, rig.opened.get())
            } finally { rig.cleanup() }
        }
    }

    @Test fun t1004HostCommitCannotHideOfferBeforeActorAdmitsLocalResource() {
        for (resource in ConnectionManagement.Resource.entries) {
            val rig = ConnectionJoinedCameraFixture(15)
            try {
                val lease = rig.protocol.offered(resource)
                rig.protocol.tablet.prepared(lease, 0); rig.protocol.pump()
                assertFalse(rig.protocol.tablet.current(lease))
                rig.commit(lease)
                assertFalse("T1004 Commit bypassed actor/resource advancement", rig.protocol.tablet.current(lease))
                assertNull(rig.protocol.tablet.nextEvent())
                synchronized(rig.gate) { rig.protocol.pump() }
                assertTrue(rig.protocol.tablet.current(lease))
                assertEquals(ConnectionSessionEvent.State(lease, ConnectionResourceState.ADMITTED), rig.protocol.tablet.nextEvent())
                assertEquals(0, rig.opened.get())
            } finally { rig.cleanup() }
        }
    }
}
