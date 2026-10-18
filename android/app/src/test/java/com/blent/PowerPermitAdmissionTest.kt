package com.blent

import com.blent.activity.DisplayActivitySource
import kotlinx.coroutines.*
import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [27, 34])
class PowerPermitAdmissionTest {
    @Test fun t1069FencedCallbackRetryKeepsTheActualOriginalSourcePermit() = runBlocking {
        val source = DisplayActivitySource()
        val snapshot = source.update
        val permit = DisplayPowerPermit { source.current(snapshot) }
        val rig = PowerRuntimeFixture()
        val owner = rig.runtime.open(StreamingPower(), permit)
        try {
            val original = checkNotNull(PowerServiceNotice.read(rig.app.starts.single()))
            source.replace()
            rig.runtime.canceled(original.notice)
            assertEquals(2, rig.app.starts.size)
            rig.attachFirst()
            assertTrue("T1069 retry discarded original source relevance", rig.native.isEmpty())
            assertTrue(rig.runtime.owns(owner))
        } finally { rig.finish(owner) }
    }
    @Test fun t1069ThousandsOfViewsKeepOneAdmissionAndApplyOnlyLatestPermit() = runBlocking {
        val source = DisplayActivitySource()
        val first = source.update
        val rig = PowerRuntimeFixture()
        val owner = rig.runtime.open(StreamingPower(), DisplayPowerPermit { source.current(first) })
        try {
            repeat(1000) {
                source.replace()
                val current = source.update
                rig.runtime.publish(owner, StreamingPower(), DisplayPowerPermit { source.current(current) })
            }
            assertEquals(1, rig.app.starts.size)
            rig.attachFirst()
            assertEquals(1, rig.native.size)
            assertTrue(rig.runtime.owns(owner))
        } finally { rig.finish(owner) }
    }
    @Test fun t1069ReentrantReaderCannotInstallWorkForAnAlreadyReplacedForeground() = runBlocking {
        val rig = PowerRuntimeFixture()
        var replacement: DisplayPowerOwner? = null
        val owner = rig.runtime.open(StreamingPower(), DisplayPowerPermit {
            replacement = rig.runtime.open(StreamingPower(true))
            true // Deliberately faulty reader mutates its publisher.
        })
        try {
            assertTrue(rig.runtime.attach(rig.port))
            val envelope = checkNotNull(PowerServiceNotice.read(rig.app.starts.single()))
            rig.runtime.receive(rig.port, envelope)
            assertFalse(rig.runtime.owns(owner))
            assertTrue(rig.runtime.owns(checkNotNull(replacement)))
            assertFalse("T1069 replaced foreground installed a dormant native workflow", rig.scope.coroutineContext[Job]!!.children.any())
            rig.drain()
        } finally { rig.finish(replacement ?: owner) }
    }
    @Test fun t1069OldIdleTimerCannotReleaseAcrossAnUndeliveredSourceViewChange() = runBlocking {
        val source = DisplayActivitySource()
        val before = source.update
        val permit = DisplayPowerPermit { source.current(before) }
        val rig = PowerRuntimeFixture()
        val owner = rig.runtime.open(StreamingPower(true, true), permit)
        try {
            rig.attachFirst()
            rig.runtime.publish(owner, StreamingPower(true, false), permit); rig.drain()
            assertEquals(1, rig.schedule.entries.size)
            source.replace()
            rig.schedule.time = WIFI_IDLE_GRACE_MILLIS; rig.schedule.due(); rig.drain()
            assertEquals("T1069 old source timer released current native work", 0, rig.settled)
            val fresh = source.update
            rig.runtime.publish(owner, StreamingPower(true, true), DisplayPowerPermit { source.current(fresh) }); rig.drain()
            assertEquals(1, rig.native.size); assertEquals(0, rig.settled)
            assertTrue(rig.schedule.entries.isEmpty())
        } finally { rig.finish(owner) }
    }
    @Test fun t1069ReaderFailureKeepsNativeUnadmittedUntilFreshDemand() = runBlocking {
        val rig = PowerRuntimeFixture()
        val owner = rig.runtime.open(StreamingPower(), DisplayPowerPermit { error("T1069 original reader unavailable") })
        try {
            rig.attachFirst()
            assertTrue(rig.native.isEmpty()); assertTrue(rig.runtime.owns(owner))
            rig.runtime.publish(owner, StreamingPower(), FixedDisplayPowerPermit); rig.drain()
            assertEquals(1, rig.native.size)
        } finally { rig.finish(owner) }
    }
    @Test fun t1069NativeRetirementAndFreshSourceRelevanceAreIndependentAdmissionGates() = runBlocking {
        val source = DisplayActivitySource()
        val first = source.update
        val rig = PowerRuntimeFixture(hold = true)
        val owner = rig.runtime.open(StreamingPower(), DisplayPowerPermit { source.current(first) })
        try {
            rig.attachFirst()
            source.replace()
            val pending = source.update
            val permit = DisplayPowerPermit { source.current(pending) }
            rig.runtime.publish(owner, StreamingPower(display = DisplayWakeState.PAUSED), permit); rig.drain()
            rig.runtime.publish(owner, StreamingPower(), permit); rig.drain()
            assertEquals(1, rig.native.size); assertEquals(0, rig.settled)
            source.replace()
            rig.release.complete(Unit); rig.drain()
            assertEquals(1, rig.settled)
            assertEquals("T1069 native fence substituted for current source relevance", 1, rig.native.size)
            val fresh = source.update
            rig.runtime.publish(owner, StreamingPower(), DisplayPowerPermit { source.current(fresh) }); rig.drain()
            withTimeout(2000) { while (rig.native.size != 2) { rig.drain(); yield() } }
            assertEquals(1, rig.settled)
        } finally { rig.finish(owner) }
    }
    @Test fun t1069QueuedPolicyCannotAdmitNativeWorkAfterItsOriginalSourceViewChanges() = runBlocking {
        val source = DisplayActivitySource()
        val original = source.update
        val permit = DisplayPowerPermit { source.current(original) }
        val rig = PowerRuntimeFixture()
        val owner = rig.runtime.open(StreamingPower(), permit)
        try {
            source.receive(JSONObject().put("status", "connected").put("display_activity",
                JSONObject().put("version", 1).put("revision", 1).put("stopped", 0).put("active", false)))
            assertFalse(permit.current())
            rig.attachFirst()
            assertTrue("T1069 queued stale view opened native wake handles", rig.native.isEmpty())
            assertTrue(rig.runtime.owns(owner))
            val current = source.update
            rig.runtime.publish(owner, StreamingPower(display = DisplayWakeState.PAUSED), DisplayPowerPermit { source.current(current) })
            rig.drain(); assertTrue(rig.native.isEmpty())
            source.receive(JSONObject().put("display_activity", JSONObject().put("version", 1).put("revision", 2).put("stopped", 0).put("active", true)))
            val resumed = source.update
            rig.runtime.publish(owner, StreamingPower(display = DisplayWakeState.ACTIVE), DisplayPowerPermit { source.current(resumed) })
            rig.drain()
            assertEquals(1, rig.native.size)
        } finally { rig.finish(owner) }
    }
}
