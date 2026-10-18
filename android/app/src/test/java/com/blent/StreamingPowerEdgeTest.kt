package com.blent

import kotlinx.coroutines.*
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [27, 34])
class StreamingPowerEdgeTest {
    @Test fun t1060MissingComponentNeverGrantsAndForeignPortCannotDetachOriginal() = runBlocking {
        val rig = PowerRuntimeFixture()
        rig.app.missing = true
        val rejected = rig.runtime.open(StreamingPower())
        assertFalse(rig.runtime.owns(rejected))
        rig.app.missing = false
        val owner = rig.runtime.open(StreamingPower())
        try {
            rig.attachFirst()
            val other = PowerRuntimeFixture()
            try {
                assertFalse(rig.runtime.attach(other.port))
                rig.runtime.detach(other.port)
                val foreign = other.runtime.open(StreamingPower())
                val envelope = checkNotNull(PowerServiceNotice.read(other.app.starts.single()))
                rig.runtime.receive(rig.port, envelope)
                other.runtime.close(foreign); other.runtime.canceled(envelope.notice)
            } finally { other.scope.cancel() }
            assertTrue(rig.runtime.owns(owner)); assertEquals(1, rig.native.size)
        } finally { rig.finish(owner) }
    }
    @Test fun t1060WifiGraceRequiresActualMatchingReadinessAndWorkerLossCannotReacquire() = runBlocking {
        val rig = PowerRuntimeFixture(ready = false)
        val owner = rig.runtime.open(StreamingPower(true, true))
        try {
            rig.attachFirst()
            rig.runtime.publish(owner, StreamingPower(true, false)); rig.drain()
            assertTrue(rig.schedule.entries.isEmpty()); assertEquals(1, rig.settled)
            rig.runtime.publish(owner, StreamingPower(true, true)); rig.drain()
            rig.events.last()(WorkerEvent.WakeReady(true, true)); rig.drain()
            assertFalse(rig.runtime.owns(owner))
            assertFalse(rig.runtime.publish(owner, StreamingPower()))
            assertEquals(2, rig.native.size)
        } finally { rig.finish(owner) }
    }
    @Test fun t1060CpuRenewalUsesOriginalBudgetAndLateReadinessCannotAffectReplacement() = runBlocking {
        val rig = PowerRuntimeFixture()
        val owner = rig.runtime.open(StreamingPower())
        try {
            rig.attachFirst()
            rig.schedule.time = CPU_WAKE_MILLIS - 1
            rig.runtime.publish(owner, StreamingPower()); rig.drain()
            assertEquals(1, rig.native.size)
            rig.schedule.time = CPU_WAKE_MILLIS
            rig.runtime.publish(owner, StreamingPower()); rig.drain()
            assertEquals(2, rig.native.size); assertEquals(1, rig.settled)
            rig.events.first()(WorkerEvent.Status("T1060 late wrong original event")); rig.drain()
            assertTrue(rig.runtime.owns(owner))
            rig.events.last()(WorkerEvent.Status("T1060 invalid current event")); rig.drain()
            assertFalse(rig.runtime.owns(owner))
        } finally { rig.finish(owner) }
    }
    @Test fun t1060ElapsedIdleReceiptReleasesWithoutWaitingForItsQueuedTimerHint() = runBlocking {
        val rig = PowerRuntimeFixture()
        val owner = rig.runtime.open(StreamingPower(true, true))
        try {
            rig.attachFirst()
            rig.runtime.publish(owner, StreamingPower(true, false)); rig.drain()
            val old = rig.schedule.entries.single().action
            rig.schedule.time = WIFI_IDLE_GRACE_MILLIS
            rig.runtime.publish(owner, StreamingPower(true, false)); rig.drain()
            assertEquals(1, rig.settled); assertTrue(rig.schedule.entries.isEmpty())
            old.run(); assertEquals(1, rig.settled)
        } finally { rig.finish(owner) }
    }
    @Test fun t1060InvalidActualQueuedClockReportsAndWithdrawsOriginalWakeAuthority() = runBlocking {
        val rig = PowerRuntimeFixture()
        val owner = rig.runtime.open(StreamingPower(true, true))
        try {
            rig.attachFirst()
            rig.runtime.publish(owner, StreamingPower(true, false)); rig.drain()
            val original = rig.schedule.entries.single().action
            rig.schedule.entries.clear(); rig.schedule.time = -1
            assertTrue(runCatching { original.run() }.isSuccess)
            rig.drain()
            assertFalse(rig.runtime.owns(owner)); assertEquals(1, rig.settled)
            assertTrue(rig.schedule.entries.isEmpty())
        } finally { rig.finish(owner) }
    }
    private suspend fun invalidClock(time: Long, idle: Boolean) {
        val rig = PowerRuntimeFixture()
        val owner = rig.runtime.open(StreamingPower(true, true))
        try {
            rig.attachFirst(); rig.schedule.time = time
            rig.runtime.publish(owner, StreamingPower(true, !idle)); rig.drain()
            assertFalse("T1060 invalid native clock retained wake authority", rig.runtime.owns(owner))
            assertTrue(rig.schedule.entries.isEmpty())
        } finally { rig.finish(owner) }
    }
    @Test fun t1060NegativeAndOverflowingClockCannotExtendNativeWake() = runBlocking {
        invalidClock(-1, false)
        invalidClock(Long.MAX_VALUE, true)
    }
}
