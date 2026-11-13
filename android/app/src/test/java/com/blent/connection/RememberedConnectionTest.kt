package com.blent.connection

import kotlinx.coroutines.cancel
import org.junit.Assert.*
import org.junit.Test

class RememberedConnectionTest {
    @Test fun t1393RememberedRetryWaitsForExactRetirementAndRetainsNoMediaConsent() {
        RememberedConnectionFixture().use { rig ->
            val original = rig.ready()
            original.ended.complete(Unit); rig.dispatcher.runCurrent()
            rig.dispatcher.advanceTo(2000)
            assertEquals("T1393: old native owner still holds the slot", 1, rig.workers.size)
            original.retired.complete(Unit); rig.dispatcher.runCurrent()
            assertEquals(2, rig.workers.size)
            assertTrue(rig.workers.last().commands.isEmpty())
            assertFalse(rig.binding.state.value.authenticated)
            assertNull(rig.binding.state.value.retryPeer)
        }
    }

    @Test fun t1393TransientFailureRetriesAtBoundedBackoffAndResetsAfterSuccess() {
        RememberedConnectionFixture().use { rig ->
            rig.end(rig.ready())
            for (deadline in listOf(2000L, 6000L, 14000L, 30000L, 60000L, 90000L)) {
                val before = rig.workers.size
                rig.dispatcher.advanceTo(deadline - 1)
                assertEquals(before, rig.workers.size)
                rig.dispatcher.advanceTo(deadline)
                assertEquals(before + 1, rig.workers.size)
                rig.end(rig.workers.last())
            }
            rig.dispatcher.advanceTo(120000)
            rig.authenticate(rig.workers.last()); rig.dispatcher.runCurrent()
            rig.end(rig.workers.last())
            assertEquals(122000L, rig.dispatcher.nextTimerAt())
        }
    }

    @Test fun t1393UnprovenAndUnrememberedFailuresNeverArmAutomaticRetry() {
        for (authenticate in listOf(false, true)) {
            RememberedConnectionFixture().use { rig ->
                val original = rig.connect()
                if (authenticate) rig.authenticate(original, remembered = false)
                rig.end(original)
                rig.dispatcher.advanceTo(60000)
                assertEquals(1, rig.workers.size)
                assertNull(rig.dispatcher.nextTimerAt())
            }
        }
    }

    @Test fun t1393PersistedFirstApprovalAndCoalescedAuthenticationRemainEligible() {
        RememberedConnectionFixture().use { rig ->
            val original = rig.connect()
            rig.authenticate(original, remembered = false)
            assertNull(rig.binding.state.value.retryPeer)
            original.report(ConnectionWorkerEvent.Trust(rig.peer, true))
            original.finish() // Do not dispatch the observer between authentication and end.
            assertEquals(rig.peer, rig.binding.state.value.retryPeer)
            rig.dispatcher.runCurrent(); rig.dispatcher.advanceTo(2000)
            assertEquals(2, rig.workers.size)
        }
    }

    @Test fun t1393ChangedOrForgottenPeerRequiresApprovalInsteadOfAnotherRetry() {
        for (peer in listOf("ab", "ef")) {
            RememberedConnectionFixture().use { rig ->
                rig.end(rig.ready()); rig.dispatcher.advanceTo(2000)
                val retry = rig.workers.last()
                val ticket = ConnectionTicket(2, ConnectionPeer.parse(peer.repeat(32)))
                retry.report(ConnectionWorkerEvent.Pairing(ticket, rig.tablet))
                rig.dispatcher.runCurrent()
                assertNotNull(rig.binding.state.value.approval)
                rig.end(retry); rig.dispatcher.advanceTo(60000)
                assertEquals(2, rig.workers.size)
                assertNull(rig.dispatcher.nextTimerAt())
            }
        }
    }

    @Test fun t1393DisconnectDuringBackoffAndInterveningManualAttemptWin() {
        for (replace in listOf(false, true)) {
            RememberedConnectionFixture().use { rig ->
                rig.end(rig.ready())
                rig.configured.disconnect()
                if (replace) rig.connect()
                val count = rig.workers.size
                rig.dispatcher.advanceTo(60000)
                assertEquals(count, rig.workers.size)
                assertNull(rig.dispatcher.nextTimerAt())
            }
        }
    }

    @Test fun t1393RevocationAndScopeLossCannotReviveAnOwnedConnection() {
        for (revoke in listOf(false, true)) {
            RememberedConnectionFixture().use { rig ->
                val original = rig.ready()
                if (revoke) assertTrue(rig.binding.revoke(checkNotNull(rig.binding.state.value.peer)))
                else rig.scope.cancel()
                rig.dispatcher.runCurrent()
                assertEquals("T1393: Activity owner loss must retain application link", 0, original.cancelled)
                rig.end(original); rig.dispatcher.advanceTo(60000)
                assertEquals(1, rig.workers.size)
                assertNull(rig.dispatcher.nextTimerAt())
            }
        }
    }

    @Test fun t1393InvalidPreferencesStopRetryWithoutImmediateLoop() {
        RememberedConnectionFixture().use { rig ->
            rig.end(rig.ready())
            rig.storage.encoded = "invalid"
            rig.dispatcher.advanceTo(60000)
            assertEquals(1, rig.workers.size)
            assertNotNull(rig.configured.state.value.failure)
            assertNull(rig.dispatcher.nextTimerAt())
        }
    }

    @Test fun t1393QuarantinedOriginalDoesNotInventAReplacementAdmission() {
        RememberedConnectionFixture().use { rig ->
            val original = rig.ready()
            original.retirementFailure = true
            rig.end(original)
            rig.dispatcher.advanceTo(60000)
            assertEquals(1, rig.workers.size)
            assertNotNull(rig.binding.state.value.cleanup)
            assertNull(rig.dispatcher.nextTimerAt())
        }
    }

    @Test fun t1393BindingCutoffDuringRetrySettingsReadWins() {
        RememberedConnectionFixture().use { rig ->
            val original = rig.ready()
            rig.scope.cancel(); rig.dispatcher.runCurrent()
            val storageDispatcher = ConnectionActorScheduler()
            val scope = kotlinx.coroutines.CoroutineScope(kotlinx.coroutines.SupervisorJob() + rig.dispatcher)
            val configured = ConfiguredConnection(rig.binding,
                ConnectionPreferenceStore(PreferenceMemoryStorage(), storageDispatcher), scope)
            try {
                configured.observeRemembered(); rig.dispatcher.runCurrent()
                rig.end(original); rig.dispatcher.advanceTo(2000)
                assertTrue(configured.state.value.pending)
                rig.binding.disconnect()
                storageDispatcher.runCurrent(); rig.dispatcher.runCurrent()
                assertEquals(1, rig.workers.size)
                assertFalse(configured.state.value.pending)
                assertNull(rig.dispatcher.nextTimerAt())
            } finally { scope.cancel(); storageDispatcher.runCurrent(); rig.dispatcher.runCurrent() }
        }
    }


    @Test fun t1393CoalescedRecoveryResetsBackoffFromCap() {
        RememberedConnectionFixture().use { rig ->
            rig.end(rig.ready())
            for (deadline in listOf(2000L, 6000L, 14000L, 30000L)) {
                rig.dispatcher.advanceTo(deadline)
                rig.end(rig.workers.last())
            }
            rig.dispatcher.advanceTo(60000)
            val recovered = rig.workers.last()
            rig.authenticate(recovered)
            recovered.finish() // Authenticated and end coalesce before the observer runs.
            rig.dispatcher.runCurrent()
            assertEquals("T1393: successful retry did not reset capped backoff", 62000L, rig.dispatcher.nextTimerAt())
        }
    }

}
