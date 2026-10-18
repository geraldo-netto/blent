package com.blent

import com.blent.DisplayWakeIntegrationTest.Rig
import kotlinx.coroutines.*
import kotlinx.coroutines.flow.MutableStateFlow
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config

/** T1058 joined policy acceptance. Native media are three independently owned
 * bounded fake runs; their retirement gates remain separate from display wake. */
@RunWith(RobolectricTestRunner::class)
@Config(sdk = [27, 34])
class DisplayWakeAcceptanceTest {
    private class Run(val name: String) : LifecycleRun {
        val released = CompletableDeferred<Unit>()
        var opened = false
        var canceled = 0
        var settled = 0
        override suspend fun execute() { opened = true; awaitCancellation() }
        override fun cancel() { canceled++ }
        override suspend fun retire() { released.await(); settled++ }
    }
    private class Auxiliary {
        val scope = CoroutineScope(SupervisorJob()+Dispatchers.Unconfined)
        val runs = ArrayList<Run>()
        private val cameras = MutableStateFlow<CameraEndpoint?>(null)
        private val microphone = MutableStateFlow<AudioEndpoint?>(null)
        private val speakers = MutableStateFlow<AudioEndpoint?>(null)
        val camera = CameraBinding({ 0 }, {}, { true }, { _, _, _, _ -> create("camera") }, cameras, scope, {})
        val mic = audio(1, microphone)
        val speaker = audio(2, speakers)
        private fun create(name: String) = Run(name).also { runs.add(it) }
        private fun audio(direction: Int, invitations: MutableStateFlow<AudioEndpoint?>) = AudioBinding(
            AudioPreferences(), {}, { true }, { _, _, _ -> create(if (direction == 1) "microphone" else "speakers") },
            invitations, scope, {}, direction, {},
        )
        fun start() {
            camera.start(); mic.start(); speaker.start()
            cameras.value = CameraEndpoint("ab".repeat(32), 12345, 640, 480, 30, 4000, CameraLens.FRONT)
            microphone.value = AudioEndpoint("ab".repeat(32), 12346, 1, 1, 40, false)
            speakers.value = AudioEndpoint("ab".repeat(32), 12347, 2, 1, 40, false)
            assertEquals(listOf("camera", "microphone", "speakers"), runs.map { it.name })
        }
        fun unchanged() {
            assertEquals(CameraLens.FRONT, camera.selected)
            assertTrue(mic.sharing); assertTrue(speaker.sharing)
            assertEquals(3, runs.size)
            runs.forEach { assertTrue(it.opened); assertEquals(0, it.canceled); assertEquals(0, it.settled) }
        }
        suspend fun close() {
            runs.forEach { it.released.complete(Unit) }
            val failures = cleanupAll(camera::shutdown, mic::shutdown, speaker::shutdown)
            checkNotNull(scope.coroutineContext[Job]).cancelAndJoin()
            assertTrue(failures.isEmpty())
            runs.forEach { assertEquals(1, it.settled) }
        }
    }
    private suspend fun finish(rig: Rig, power: PowerRuntimeFixture, binding: StreamingPowerBinding,
                               window: WindowPowerBinding, owner: DisplayPowerOwner) {
        window.stop(); binding.stop()
        try { rig.close() }
        finally { power.finish(owner); checkNotNull(power.scope.coroutineContext[Job]).cancelAndJoin() }
    }
    @Test fun t1058DisplayCyclesAndBackgroundPreserveThreeOriginalAuxiliaryOwners() = runBlocking {
        val rig = Rig()
        val power = PowerRuntimeFixture()
        val binding = StreamingPowerBinding(power.app, power.runtime)
        val writes = ArrayList<Boolean>()
        val window = WindowPowerBinding(writes::add)
        val auxiliary = Auxiliary()
        binding.start(rig.session.powerDemandNow(), rig.session.powerDemandUpdates())
        window.start(rig.session.powerDemandNow(), rig.session.powerDemandUpdates())
        val owner = checkNotNull(PowerServiceNotice.read(power.app.starts.single())).notice.original.owner
        try {
            auxiliary.start(); rig.start(); rig.signal(true)
            rig.until { writes.last() }; power.attachFirst()
            rig.until(power::drain) { power.native.size == 1 }
            repeat(8) { turn ->
                val admitted = power.native.size
                rig.signal(false, 2L+turn*2, 1L+turn, false)
                rig.until(power::drain) { !writes.last() && power.settled == power.native.size }
                auxiliary.unchanged()
                rig.session.handle(SettingsEvent.BatterySaver(true))
                rig.drain(); power.drain(); assertFalse(writes.last())
                rig.session.handle(SettingsEvent.BatterySaver(false))
                rig.signal(true, 3L+turn*2, 1L+turn, false)
                rig.until(power::drain) { writes.last() && power.native.size == admitted+1 }
                auxiliary.unchanged()
            }
            rig.session.stop()
            rig.until(power::drain) { !writes.last() && power.settled == power.native.size }
            auxiliary.unchanged()
            auxiliary.mic.stopSharing()
            assertTrue(auxiliary.runs[1].canceled > 0); assertEquals(0, auxiliary.runs[1].settled)
            assertTrue(auxiliary.speaker.sharing); assertEquals(CameraLens.FRONT, auxiliary.camera.selected)
            auxiliary.runs[1].released.complete(Unit)
            rig.until { auxiliary.runs[1].settled == 1 }
            assertEquals(0, auxiliary.runs[0].canceled); assertEquals(0, auxiliary.runs[2].canceled)
        } finally {
            try { finish(rig, power, binding, window, owner) } finally { auxiliary.close() }
        }
    }
    @Test fun t1058QueuedNativeRequestsCannotCrossReplacementOrMalformedSourceCutoff() = runBlocking {
        val rig = Rig()
        val power = PowerRuntimeFixture()
        val binding = StreamingPowerBinding(power.app, power.runtime)
        val writes = ArrayList<Boolean>()
        val window = WindowPowerBinding(writes::add)
        binding.start(rig.session.powerDemandNow(), rig.session.powerDemandUpdates())
        window.start(rig.session.powerDemandNow(), rig.session.powerDemandUpdates())
        val owner = checkNotNull(PowerServiceNotice.read(power.app.starts.single())).notice.original.owner
        try {
            rig.start(); rig.signal(true); rig.until { writes.last() }
            val old = rig.sockets.last()
            val permit = rig.session.powerDemandNow().permit
            rig.control.disconnect(); rig.start(); rig.signal(false)
            rig.until { !writes.last() }
            val current = rig.control.displayActivityUpdate
            old.second.onMessage(old.first, """{"status":"connected","pen_only":false}""")
            assertSame(current, rig.control.displayActivityUpdate); assertFalse(permit.current())
            power.attachFirst(); assertTrue(power.native.isEmpty())
            rig.signal(true, 2, 0, false)
            rig.until { writes.last() && power.notices.isNotEmpty() }
            val queued = rig.session.powerDemandNow().permit
            rig.signal(true, -1, 0, false)
            assertFalse(queued.current())
            power.drain()
            rig.until(power::drain) { !writes.last() }
            assertEquals(DisplayWakeState.UNAVAILABLE, rig.session.powerNow().display)
            rig.session.handle(SettingsEvent.BatterySaver(false)); rig.drain(); power.drain()
            assertTrue("T1058 stale native receipt admitted a wake worker", power.native.isEmpty())
            assertTrue(rig.control.displayActivityUpdate.supportKnown)
        } finally { finish(rig, power, binding, window, owner) }
    }
}
