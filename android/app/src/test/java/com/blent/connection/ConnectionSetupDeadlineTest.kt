package com.blent.connection

import java.io.IOException
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicLong
import org.junit.Assert.*
import org.junit.Test

/** T1094: snapshots observe retained original budgets; no live USB or TLS. */
class ConnectionSetupDeadlineTest {
    @Test fun t1094SetupStagesKeepOriginalDiscoveryApprovalAndAuthenticationBudgets() {
        val backend = SetupBackend().apply {
            discoverGate = CountDownLatch(1); readyGate = CountDownLatch(1); authenticateGate = CountDownLatch(1)
        }
        SetupRig(backend).use { rig ->
            assertTrue(backend.discoverEntered.await(2, TimeUnit.SECONDS))
            assertEquals(ConnectionDeadline.at(110), rig.owner.nextDeadline())
            repeat(128) { assertEquals(ConnectionDeadline.at(110), rig.owner.nextDeadline()) }
            backend.discoverGate.countDown()
            val ticket = rig.ticket()
            assertEquals(ConnectionDeadline.at(1010), rig.owner.nextDeadline())
            rig.clock.set(20); rig.owner.decide(ticket, true, false)
            assertTrue(backend.readyEntered.await(2, TimeUnit.SECONDS))
            assertEquals(ConnectionDeadline.at(1010), rig.owner.nextDeadline())
            backend.readyGate.countDown()
            assertTrue(backend.entered.await(2, TimeUnit.SECONDS))
            assertEquals(ConnectionDeadline.at(120), rig.owner.nextDeadline())
            backend.authenticateGate.countDown(); rig.ready()
            // Preserve the implementation's effective clock-exhaustion limit.
            assertEquals(ConnectionDeadline.at(Long.MAX_VALUE), rig.owner.nextDeadline())
            assertFalse(rig.owner.retired())
        }
    }

    @Test fun t1094StoppedSetupBudgetCannotSettleOriginalBlockedDiscovery() {
        val backend = SetupBackend().apply { discoverGate = CountDownLatch(1) }
        SetupRig(backend).use { rig ->
            assertTrue(backend.discoverEntered.await(2, TimeUnit.SECONDS))
            rig.owner.stop()
            assertEquals(ConnectionDeadline.at(60), rig.owner.nextDeadline())
            rig.clock.set(60); rig.owner.advance()
            assertEquals(ConnectionDeadline.IDLE, rig.owner.nextDeadline())
            assertTrue(rig.owner.terminationRequired()); assertFalse(rig.owner.retired())
            backend.discoverGate.countDown(); setupAwait { rig.owner.retired() }
            assertEquals(ConnectionDeadline.IDLE, rig.owner.nextDeadline())
            assertNull(rig.owner.takeReady())
        }
    }

    @Test fun t1094ActualSetupRetirementRemovesUnexpiredBudget() {
        SetupRig().use { rig ->
            rig.ticket(); rig.owner.stop(); setupAwait { rig.owner.retired() }
            assertEquals(ConnectionDeadline.IDLE, rig.owner.nextDeadline())
        }
    }

    @Test fun t1094SetupCloseFailureReportsNoNewBudgetWithoutRetirementProof() {
        val backend = SetupBackend().apply { closeFailure = IOException("T1094 simulated close failure") }
        SetupRig(backend).use { rig ->
            try {
                rig.ticket(); rig.owner.stop(); setupAwait { rig.owner.terminationRequired() }
                assertEquals(ConnectionDeadline.IDLE, rig.owner.nextDeadline())
                assertFalse(rig.owner.retired())
            } finally {
                backend.closeFailure = null; backend.close()
                setupAwait { rig.owner.retired() }
            }
        }
    }

    @Test fun t1094ActualMaximumSetupBudgetIsNeverMisreportedAsIdle() {
        val backend = SetupBackend().apply { discoverGate = CountDownLatch(1) }
        val clock = AtomicLong(Long.MAX_VALUE - 100)
        val store = SetupStore()
        val owner = ConnectionSetupOwner(backend, { store }, clock::get, ConnectionSetupLimits(4096, 100, 100, 50))
        try {
            assertTrue(backend.discoverEntered.await(2, TimeUnit.SECONDS))
            assertEquals(ConnectionDeadline.at(Long.MAX_VALUE), owner.nextDeadline())
        } finally {
            owner.stop(); backend.discoverGate.countDown(); setupAwait { owner.retired() }
        }
    }
}
