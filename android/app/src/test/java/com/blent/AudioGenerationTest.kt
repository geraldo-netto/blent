package com.blent

import kotlinx.coroutines.*
import kotlinx.coroutines.flow.MutableStateFlow
import org.junit.Assert.*
import org.junit.Test
import java.math.BigInteger

/** T1183: independent microphone/speaker epochs do not release native claims. */
class AudioGenerationTest {
    private class Worker(val report: (String) -> Unit, immediate: Boolean) : LifecycleRun {
        val finish = CompletableDeferred<Unit>()
        val release = CompletableDeferred<Unit>().also { if (immediate) it.complete(Unit) }
        var retired = 0
        override suspend fun execute() { finish.await() }
        override fun cancel() {}
        override suspend fun retire() { release.await(); retired++ }
    }
    private class Authority : MediaConsumerAuthority {
        var available = true
        val claims = mutableListOf<Claim>()
        inner class Claim : MediaConsumerClaim {
            val proofs = mutableListOf<ResourceRetirement>()
            override fun current() = available
            override fun retired(proof: ResourceRetirement) { proofs.add(proof) }
        }
        override fun retain(): MediaConsumerClaim? =
            if (available) Claim().also { claims.add(it) } else null
        override fun stop(): Boolean { available = false; return true }
    }
    private class Rig(seed: Long, val direction: Int, immediate: Boolean = false) : AutoCloseable {
        val owner = SupervisorJob()
        val scope = CoroutineScope(owner + Dispatchers.Unconfined)
        val invitations = MutableStateFlow<AudioEndpoint?>(null)
        val workers = mutableListOf<Worker>()
        val service = mutableListOf<String?>()
        val saved = mutableListOf<AudioPreferences>()
        val endpoint = AudioEndpoint("ab".repeat(32), 12345, direction, 1, 40, true)
        var allowed = true
        var prompts = 0
        var serviceHook: (String?) -> Unit = {}
        val binding = AudioBinding(AudioPreferences(background = true), { prompts++ }, { allowed }, { _, _, report ->
            Worker(report, immediate).also { workers.add(it) }
        }, invitations, scope, { service.add(it); serviceHook(it) }, direction, saved::add, seed)
        fun invite(value: AudioEndpoint = endpoint) { binding.start(); invitations.value = value }
        override fun close() {
            binding.stopSharing()
            workers.forEach { it.release.complete(Unit) }
            binding.shutdown()
            owner.cancel()
            runBlocking { owner.join() }
        }
    }

    @Test fun t1183TerminalAdmissionPrecedesPermissionServiceAndNativeEffects() {
        for (direction in 1..2) Rig(Long.MAX_VALUE, direction).use { rig ->
            rig.allowed = false
            rig.invite()
            assertEquals("T1183 terminal invitation prompted permission", 0, rig.prompts)
            assertNull(rig.binding.permissionPending)
            assertNull(rig.binding.pending)
            rig.allowed = true
            rig.binding.accept()
            assertTrue("T1183 terminal invitation started native worker", rig.workers.isEmpty())
            assertTrue(rig.service.isEmpty())
            assertTrue(rig.binding.status.contains("generations exhausted"))
        }
    }

    @Test fun t1183TerminalExternalTargetClosesWithoutRetainOrService() {
        for (direction in 1..2) Rig(Long.MAX_VALUE, direction).use { rig ->
            val authority = Authority()
            rig.binding.start()
            rig.binding.useTarget(AudioConsumerTarget(rig.endpoint, authority))
            assertFalse(authority.available)
            assertTrue(authority.claims.isEmpty())
            assertTrue(rig.workers.isEmpty())
            assertTrue(rig.service.isEmpty())
            assertTrue(rig.binding.status.contains("generations exhausted"))
        }
    }

    @Test fun t1183LastGrantRetainsOriginalClaimThroughConfigureAndDeniedReplacement() {
        for (direction in 1..2) Rig(Long.MAX_VALUE - 1, direction).use { rig ->
            val authority = Authority()
            rig.binding.start()
            rig.binding.useTarget(AudioConsumerTarget(rig.endpoint, authority))
            val original = rig.workers.single()
            val claim = authority.claims.single()
            original.report("current last grant")
            assertEquals("current last grant", rig.binding.status)
            val preferences = AudioPreferences(background = true, gain = 50)
            rig.binding.configure(preferences)
            assertEquals(listOf(preferences), rig.saved)
            assertEquals(preferences, rig.binding.preferences)
            assertFalse(authority.available)
            assertTrue(claim.proofs.isEmpty())
            rig.binding.useTarget(null)
            repeat(3) { rig.invite() }
            assertEquals("T1183 exhausted replacement acquired another run", 1, rig.workers.size)
            assertFalse(rig.binding.sharing)
            assertNull(rig.service.last())
            original.report("stale before retirement")
            assertFalse(rig.binding.status.contains("stale"))
            original.release.complete(Unit)
            assertEquals(listOf(ResourceRetirement.RETIRED), claim.proofs)
            assertEquals(1, original.retired)
            assertEquals(1, rig.workers.size)
            original.report("stale after retirement")
            assertFalse(rig.binding.status.contains("stale"))
        }
    }

    @Test fun t1183ExecutionEndRevokesLateStatusBeforeActualNativeRetirement() {
        for (direction in 1..2) Rig(Long.MAX_VALUE - 1, direction).use { rig ->
            rig.invite()
            val original = rig.workers.single()
            original.finish.complete(Unit)
            assertFalse(rig.binding.sharing)
            assertNull(rig.service.last())
            assertEquals(0, original.retired)
            original.report("stale execution ended")
            assertFalse("T1183 ended audio kept callback authority", rig.binding.status.contains("stale"))
            original.release.complete(Unit)
            original.report("stale native retired")
            assertFalse(rig.binding.status.contains("stale"))
        }
    }

    @Test fun t1183OriginalPermissionCannotConsumeLastGrantOrReviveAfterExhaustion() {
        Rig(Long.MAX_VALUE - 1, 1).use { rig ->
            rig.allowed = false
            rig.invite()
            val old = rig.binding.permissionPending!!
            rig.invite()
            val latest = rig.binding.permissionPending!!
            rig.allowed = true
            rig.binding.permissionResult(true, old)
            assertSame(latest, rig.binding.permissionPending)
            assertTrue(rig.workers.isEmpty())
            rig.binding.permissionResult(true, latest)
            assertEquals(1, rig.workers.size)
            rig.allowed = false
            val prompts = rig.prompts
            rig.invite()
            assertEquals("T1183 exhausted invitation prompted again", prompts, rig.prompts)
            assertNull(rig.binding.permissionPending)
            rig.allowed = true
            rig.binding.permissionResult(true, latest)
            assertEquals(1, rig.workers.size)
            assertFalse(rig.binding.sharing)
        }
    }

    @Test fun t1183ExhaustedMicrophoneCannotRetireIndependentSpeaker() {
        Rig(Long.MAX_VALUE - 1, 1).use { microphone -> Rig(0, 2).use { speaker ->
            microphone.invite(); speaker.invite()
            val originalSpeaker = speaker.workers.single()
            val originalService = speaker.service.single()
            microphone.invite()
            assertTrue(speaker.binding.sharing)
            assertTrue(speaker.binding.ownsBackground(originalService))
            assertEquals(0, originalSpeaker.retired)
            assertEquals(1, speaker.workers.size)
            originalSpeaker.report("speaker still current")
            assertEquals("speaker still current", speaker.binding.status)
        } }
    }

    @Test fun t1183ServiceWithdrawalCannotAdmitWorkerAfterOriginalRequestRevoked() {
        for (direction in 1..2) Rig(Long.MAX_VALUE - 1, direction).use { rig ->
            rig.serviceHook = { run -> if (run != null) rig.binding.stopSharing() }
            rig.invite()
            assertTrue("T1183 revoked request started native worker", rig.workers.isEmpty())
            assertFalse(rig.binding.sharing)
            assertEquals(2, rig.service.size)
            assertNull(rig.service.last())
        }
    }

    @Test fun t1183BoundedTwoDirectionStartsAndStopsMatchWideAdmissionModel() {
        val maximum = BigInteger.valueOf(Long.MAX_VALUE)
        for (direction in 1..2) {
            for (seed in (0L..16L) + (Long.MAX_VALUE - 16..Long.MAX_VALUE)) {
                Rig(seed, direction, immediate = true).use { rig ->
                    var expected = BigInteger.valueOf(seed)
                    var starts = 0
                    repeat(20) {
                        if (expected < maximum) { expected += BigInteger.ONE; starts++ }
                        rig.invite()
                        assertEquals("T1183 direction=$direction seed=$seed attempt=$it", starts, rig.workers.size)
                        rig.binding.stopSharing()
                    }
                    assertEquals(starts, rig.workers.sumOf { it.retired })
                }
            }
        }
    }

    @Test fun t1183NegativeSeedsCannotAdmitEitherDirection() {
        val owner = SupervisorJob()
        val scope = CoroutineScope(owner + Dispatchers.Unconfined)
        try {
            for (direction in 1..2) for (seed in listOf(Long.MIN_VALUE, -17L, -1L)) {
                assertThrows(IllegalArgumentException::class.java) {
                    AudioBinding(AudioPreferences(), {}, { true }, { _, _, _ -> error("T1183 native effect") },
                        MutableStateFlow(null), scope, {}, direction, {}, seed)
                }
            }
        } finally { owner.cancel(); runBlocking { owner.join() } }
    }
}
