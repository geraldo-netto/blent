package com.blent.connection

import org.junit.Assert.*
import org.junit.Test

class ConnectionRuntimeDeadlineTest {
    @Test fun t1095LocalRunRetainsTransportDeadlineWhenNativeCapabilityIsUnknown() {
        val rig = ResourceRig()
        val raw = RuntimeRun()
        val transport = object : ConnectionRun by raw, ConnectionDeadlineSource {
            override fun nextDeadline(): ConnectionDeadline = ConnectionDeadline.at(30)
        }
        val run = ConnectionLocalRun(transport, rig.local)
        assertEquals(ConnectionDeadline.at(30), run.nextDeadline())
        rig.offer()
        assertEquals(ConnectionDeadline(30, false), run.nextDeadline())
        rig.local.stop(0); rig.native.forEach { it.finish() }; rig.local.advance(1)
        assertEquals(ConnectionDeadline.at(30), run.nextDeadline())
        assertEquals(ConnectionDeadline.UNKNOWN, ConnectionLocalRun(raw, rig.local).nextDeadline())
    }

    @Test fun t1095RuntimeCombinesOnlyInstalledOwnersAndKeepsUnknownBudgetsExplicit() {
        val rawSetup = RuntimeSetup()
        val setup = object : ConnectionSetup by rawSetup, ConnectionDeadlineSource {
            override fun nextDeadline(): ConnectionDeadline = ConnectionDeadline.at(80)
        }
        val rawRun = RuntimeRun()
        val run = object : ConnectionRun by rawRun, ConnectionDeadlineSource {
            override fun nextDeadline(): ConnectionDeadline = ConnectionDeadline.at(20)
        }
        val owner = ConnectionRuntimeOwner(setup, ConnectionRunFactory { run }, { 0 })
        assertEquals(ConnectionDeadline.at(80), owner.nextDeadline())
        rawSetup.offer(); owner.advance()
        assertEquals(ConnectionDeadline.at(20), owner.nextDeadline())
        val unknown = ConnectionRuntimeOwner(RuntimeSetup(), ConnectionRunFactory { error("T1095 source-only fixture") }, { 0 })
        assertEquals(ConnectionDeadline.UNKNOWN, unknown.nextDeadline())
    }

    @Test fun t1095FactoryUncertaintyKeepsKnownOriginalSetupBudgetWithoutClaimingRetirement() {
        val raw = RuntimeSetup()
        val setup = object : ConnectionSetup by raw, ConnectionDeadlineSource {
            override fun nextDeadline(): ConnectionDeadline = ConnectionDeadline.at(80)
        }
        val owner = ConnectionRuntimeOwner(setup, ConnectionRunFactory { error("T1095 partial native factory") }, { 0 })
        raw.offer(); owner.advance()
        assertEquals(ConnectionDeadline(80, false), owner.nextDeadline())
        assertTrue(owner.terminationRequired()); assertFalse(owner.retired())
    }
}
