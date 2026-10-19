package com.blent.connection

import org.junit.Assert.*
import org.junit.Test

class ConnectionAdmissionWaitTest {
    @Test fun t1092KnownIdleAdmissionSleepsAndUnknownProviderRetainsLegacyPoll() {
        AdmissionWaitRig().use { rig ->
            rig.start(); assertEquals(1000L, rig.scheduler.nextTimerAt())
            val before = rig.state.ticks.get()
            rig.scheduler.advanceTo(999); assertEquals(before, rig.state.ticks.get())
        }
        AdmissionWaitRig(known = false).use { rig ->
            rig.start(); assertEquals(10L, rig.scheduler.nextTimerAt())
        }
    }

    @Test fun t1092AdmissionObservesNearestDeadlineBeforeLongIdleFallback() {
        AdmissionWaitRig().use { rig ->
            rig.budget = ConnectionDeadline.at(5); rig.start()
            assertEquals(5L, rig.scheduler.nextTimerAt())
            rig.scheduler.advanceTo(5)
            assertTrue(rig.actor.execution.isCompleted); assertNotNull(rig.actor.outcome())
        }
    }

    @Test fun t1092SpuriousAdmissionHintsPreserveOriginalDeadline() {
        AdmissionWaitRig().use { rig ->
            rig.budget = ConnectionDeadline.at(100); rig.start(); rig.scheduler.advanceTo(50)
            repeat(64) { checkNotNull(rig.wake)() }; rig.scheduler.runCurrent()
            assertEquals(100L, rig.scheduler.nextTimerAt())
        }
    }

    @Test fun t1092MissingAdmissionHintCannotHideShortConfiguredBudget() {
        AdmissionWaitRig(ConnectionWorkerSpec("fixture", timerMs = 1000, nativeTimeoutMs = 5)).use { rig ->
            rig.start(); assertEquals(5L, rig.scheduler.nextTimerAt())
            rig.scheduler.advanceTo(1); rig.budget = ConnectionDeadline.at(6)
            rig.scheduler.advanceTo(5); assertEquals(6L, rig.scheduler.nextTimerAt())
            rig.scheduler.advanceTo(6)
            assertTrue(rig.actor.execution.isCompleted); assertNotNull(rig.actor.outcome())
        }
    }

    @Test fun t1092AdmissionRetirementWaitsForOriginalPermissionOwnerProof() {
        AdmissionWaitRig().use { rig ->
            rig.state.stopRetires = false; rig.start(); rig.actor.run.cancel(); rig.scheduler.runCurrent()
            assertEquals(10L, rig.scheduler.nextTimerAt()); assertFalse(rig.actor.execution.isCompleted)
            rig.scheduler.advanceTo(10); assertFalse(rig.actor.execution.isCompleted)
            rig.state.gone = true; checkNotNull(rig.wake)(); rig.scheduler.runCurrent()
            assertTrue(rig.actor.execution.isCompleted)
        }
    }
}
