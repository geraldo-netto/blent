package com.blent

import kotlinx.coroutines.*
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [27, 34])
class StreamingPowerRuntimeTest {
    @Test fun t1060ReplacementAndOldStopsKeepOneOriginalNativeAdmission() = runBlocking {
        val rig = PowerRuntimeFixture(hold = true)
        val old = rig.runtime.open(StreamingPower())
        val fresh = rig.runtime.open(StreamingPower(true, true, StreamTransport.NETWORK))
        try {
            assertEquals(1, rig.app.starts.size)
            rig.runtime.close(old)
            assertTrue(rig.runtime.owns(fresh))
            rig.attachFirst()
            assertEquals(listOf(MediaTask.Wake(false, true)), rig.native)
            assertFalse(rig.runtime.owns(old))
            assertTrue(rig.runtime.publish(fresh, StreamingPower(display = DisplayWakeState.PAUSED)))
            rig.drain()
            assertEquals(0, rig.settled)
            assertTrue(rig.canceled > 0)
            assertTrue(rig.runtime.publish(fresh, StreamingPower()))
            rig.drain()
            assertEquals("T1060 unresolved original process was replaced", 1, rig.native.size)
            rig.events.first()(WorkerEvent.WakeReady(true, true)); rig.drain()
            assertTrue(rig.runtime.owns(fresh))
            rig.release.complete(Unit)
            withTimeout(2000) { while (rig.native.size != 2) { rig.drain(); yield() } }
            assertEquals(1, rig.settled)
        } finally { rig.finish(fresh) }
    }
    @Test fun t1060KnownRejectionAndAmbiguousAcceptedStartRemainDistinct() = runBlocking {
        val rejected = PowerRuntimeFixture()
        rejected.app.denied = true
        val noGrant = rejected.runtime.open(StreamingPower())
        assertFalse(rejected.runtime.owns(noGrant)); assertTrue(rejected.app.starts.isEmpty())
        rejected.app.denied = false
        val retry = rejected.runtime.open(StreamingPower())
        try { rejected.attachFirst(); assertEquals(1, rejected.native.size) }
        finally { rejected.finish(retry) }

        val unknown = PowerRuntimeFixture()
        unknown.app.ambiguous = true
        val lost = unknown.runtime.open(StreamingPower())
        val replacement = unknown.runtime.open(StreamingPower(true, true))
        try {
            assertFalse(unknown.runtime.owns(lost))
            assertEquals("T1060 unknown handoff discarded original admission", 1, unknown.app.starts.size)
            unknown.app.ambiguous = false
            unknown.attachFirst()
            assertEquals(listOf(MediaTask.Wake(false, true)), unknown.native)
        } finally { unknown.finish(replacement) }
    }
    @Test fun t1060NativeGraceUsesActualReadinessAndAbsoluteOriginalDeadline() = runBlocking {
        val rig = PowerRuntimeFixture()
        val owner = rig.runtime.open(StreamingPower(true, true, StreamTransport.NETWORK))
        try {
            rig.attachFirst(); assertEquals(1, rig.native.size)
            rig.runtime.publish(owner, StreamingPower(true, false)); rig.drain()
            assertEquals(1, rig.native.size)
            assertEquals(5000L, rig.schedule.entries.single().due)
            rig.schedule.time = 4000
            rig.runtime.publish(owner, StreamingPower(true, false)); rig.drain()
            assertEquals(5000L, rig.schedule.entries.single().due)
            rig.schedule.time = 5000; rig.schedule.due(); rig.drain()
            assertEquals(1, rig.settled)
            rig.runtime.publish(owner, StreamingPower(true, true, StreamTransport.NETWORK)); rig.drain()
            assertEquals(2, rig.native.size)
            rig.runtime.publish(owner, StreamingPower(true, false, StreamTransport.USB)); rig.drain()
            assertEquals(2, rig.settled)
            assertTrue(rig.schedule.entries.isEmpty())
        } finally { rig.finish(owner) }
    }
    @Test fun t1060ForeignAndMalformedOrDuplicateNoticesCannotRetireCurrentReplacement() = runBlocking {
        val rig = PowerRuntimeFixture()
        val owner = rig.runtime.open(StreamingPower())
        try {
            val original = checkNotNull(PowerServiceNotice.read(rig.app.starts.single()))
            rig.attachFirst()
            rig.runtime.publish(owner, StreamingPower(true, true)); rig.drain()
            val count = rig.native.size
            rig.runtime.receive(rig.port, original, false)
            assertTrue(rig.runtime.owns(owner))
            assertEquals(count, rig.native.size)
            rig.runtime.publish(owner, StreamingPower())
            val current = checkNotNull(rig.notices.poll())
            rig.runtime.receive(rig.port, PowerServiceNotice(rig.runtime, current), false)
            rig.drain()
            assertFalse(rig.runtime.owns(owner))
        } finally { rig.finish(owner) }
    }
}
