package com.blent

import kotlinx.coroutines.*
import kotlinx.coroutines.flow.MutableStateFlow
import org.junit.Assert.*
import org.junit.Test
import java.math.BigInteger

/** T1182: callback identity and original native retirement are separate owners. */
class CameraGenerationTest {
    private val endpoint = CameraEndpoint("ab".repeat(32), 12345, 640, 480, 30, 1000, CameraLens.FRONT)

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

    private inner class Rig(seed: Long, val immediate: Boolean = false) : AutoCloseable {
        val owner = SupervisorJob()
        val scope = CoroutineScope(owner + Dispatchers.Unconfined)
        val invitations = MutableStateFlow<CameraEndpoint?>(null)
        val workers = mutableListOf<Worker>()
        val background = mutableListOf<String?>()
        var allowed = true
        var prompts = 0
        var backgroundHook: (String?) -> Unit = {}
        val binding = CameraBinding({ 0 }, { prompts++ }, { allowed }, { _, _, _, report ->
            Worker(report, immediate).also { workers.add(it) }
        }, invitations, scope, { background.add(it); backgroundHook(it) }, seed)
        fun invite(value: CameraEndpoint = endpoint) { binding.start(); invitations.value = value }
        override fun close() {
            binding.shutdown()
            workers.forEach { it.release.complete(Unit) }
            owner.cancel()
            runBlocking { owner.join() }
        }
    }

    @Test fun t1182TerminalAdmissionPrecedesPermissionBackgroundAndNativeEffects() {
        Rig(Long.MAX_VALUE).use { rig ->
            rig.allowed = false
            rig.invite(endpoint.copy(background = true))
            assertEquals("T1182 terminal admission prompted permission", 0, rig.prompts)
            assertNull(rig.binding.permissionPending)
            assertTrue(rig.workers.isEmpty())
            assertTrue(rig.background.isEmpty())
            assertTrue(rig.binding.status.contains("generations exhausted"))
            rig.allowed = true
            rig.binding.choose(CameraLens.REAR)
            assertTrue("T1182 terminal generation acquired native run", rig.workers.isEmpty())
        }
    }

    @Test fun t1182LastGrantRejectsReplacementButRetainsOriginalClaimUntilNativeCleanup() {
        Rig(Long.MAX_VALUE - 1).use { rig ->
            val authority = Authority()
            rig.binding.start()
            rig.binding.useTarget(CameraCaptureTarget(endpoint.copy(background = true), authority))
            val original = rig.workers.single()
            val claim = authority.claims.single()
            assertNotNull(rig.background.single())
            original.report("current last grant")
            assertEquals("current last grant", rig.binding.status)
            repeat(3) { rig.binding.choose(CameraLens.REAR) }
            assertEquals("T1182 exhausted replacement retained another claim", 1, authority.claims.size)
            assertEquals(1, rig.workers.size)
            assertNull(rig.binding.selected)
            assertFalse(authority.available)
            assertNull(rig.background.last())
            assertTrue("T1182 callback revocation released native claim", claim.proofs.isEmpty())
            original.report("stale after terminal rejection")
            assertFalse(rig.binding.status.contains("stale"))
            original.release.complete(Unit)
            assertEquals(listOf(ResourceRetirement.RETIRED), claim.proofs)
            assertEquals(1, original.retired)
            assertEquals(1, rig.workers.size)
            original.report("stale after native retirement")
            assertFalse(rig.binding.status.contains("stale"))
        }
    }

    @Test fun t1182ExecutionEndRevokesCallbacksBeforeHeldRetirement() {
        Rig(Long.MAX_VALUE - 1).use { rig ->
            rig.invite(endpoint.copy(background = true))
            val original = rig.workers.single()
            original.report("current")
            assertEquals("current", rig.binding.status)
            original.finish.complete(Unit)
            assertNull(rig.binding.selected)
            assertNull(rig.background.last())
            assertEquals(0, original.retired)
            original.report("stale execution-ended callback")
            assertFalse("T1182 ended run kept publication authority", rig.binding.status.contains("stale"))
            original.release.complete(Unit)
            original.report("stale retired callback")
            assertFalse(rig.binding.status.contains("stale"))
        }
    }

    @Test fun t1182OldPermissionCannotConsumeLastGrantOrReviveAfterPermissionLoss() {
        Rig(Long.MAX_VALUE - 1).use { rig ->
            rig.allowed = false
            rig.invite()
            val old = rig.binding.permissionPending!!
            rig.binding.choose(CameraLens.REAR)
            val latest = rig.binding.permissionPending!!
            assertNotSame(old, latest)
            rig.allowed = true
            rig.binding.permissionResult(true, old)
            assertSame(latest, rig.binding.permissionPending)
            assertTrue(rig.workers.isEmpty())
            rig.binding.permissionResult(true, latest)
            assertEquals(CameraLens.REAR, rig.binding.selected)
            rig.allowed = false
            rig.binding.permissionChanged()
            val prompts = rig.prompts
            rig.binding.choose(CameraLens.FRONT)
            assertEquals("T1182 exhausted owner prompted again", prompts, rig.prompts)
            rig.allowed = true
            rig.binding.permissionResult(true, latest)
            assertEquals(1, rig.workers.size)
            assertNull(rig.binding.selected)
        }
    }

    @Test fun t1182TerminalTargetStopsOriginalWithoutRetainOrPermissionPrompt() {
        Rig(Long.MAX_VALUE).use { rig ->
            rig.allowed = false
            val authority = Authority()
            rig.binding.useTarget(CameraCaptureTarget(endpoint, authority))
            assertTrue(authority.available)
            rig.binding.start()
            assertFalse("T1182 terminal target remained active", authority.available)
            assertTrue(authority.claims.isEmpty())
            assertEquals(0, rig.prompts)
            assertTrue(rig.workers.isEmpty())
            rig.binding.useTarget(null)
            rig.binding.permissionResult(true)
            assertTrue(rig.workers.isEmpty())
        }
    }

    @Test fun t1182BoundedStartsAndStopsMatchWiderIntegerAdmission() {
        val maximum = BigInteger.valueOf(Long.MAX_VALUE)
        for (seed in (0L..16L) + (Long.MAX_VALUE - 16..Long.MAX_VALUE)) {
            Rig(seed, immediate = true).use { rig ->
                rig.invite(endpoint.copy(requestedLens = null))
                var expected = BigInteger.valueOf(seed)
                var starts = 0
                repeat(40) {
                    if (expected < maximum) { expected += BigInteger.ONE; starts++ }
                    rig.binding.choose(CameraLens.FRONT)
                    assertEquals("T1182 seed=$seed attempt=$it", starts, rig.workers.size)
                    rig.binding.choose(null)
                }
                assertEquals(starts, rig.workers.sumOf { it.retired })
            }
        }
    }

    @Test fun t1182BackgroundWithdrawalCannotAdmitWorkerAfterOriginalRequestRevoked() {
        Rig(Long.MAX_VALUE - 1).use { rig ->
            rig.backgroundHook = { run -> if (run != null) rig.binding.choose(null) }
            rig.invite(endpoint.copy(background = true))
            assertTrue("T1182 revoked request started a native worker", rig.workers.isEmpty())
            assertNull(rig.binding.selected)
            assertEquals(2, rig.background.size)
            assertNull(rig.background.last())
            rig.binding.choose(CameraLens.FRONT)
            assertTrue(rig.workers.isEmpty())
        }
    }

    @Test fun t1200FixtureCleanupCutsPendingAdmissionBeforeReleasingNativeGate() {
        val rig = Rig(0)
        try {
            rig.invite()
            rig.binding.choose(CameraLens.REAR)
            assertEquals(1, rig.workers.size)
            val failure = runCatching { rig.close() }.exceptionOrNull()
            assertNull("T1200 fixture failed to retire original scope: $failure", failure)
            assertEquals("T1200 cleanup opened queued native replacement", 1, rig.workers.size)
            assertEquals(1, rig.workers.single().retired)
        } finally {
            // Independent emergency sponsor also settles the deliberate faulty control.
            rig.binding.shutdown()
            rig.workers.toList().forEach { it.release.complete(Unit) }
            rig.owner.cancel()
            runBlocking { rig.owner.join() }
        }
    }

    @Test fun t1182NegativeSeedsCannotCreateBindingAuthority() {
        val owner = SupervisorJob()
        val scope = CoroutineScope(owner + Dispatchers.Unconfined)
        try {
            for (seed in listOf(Long.MIN_VALUE, -17L, -1L)) {
                assertThrows(IllegalArgumentException::class.java) {
                    CameraBinding({ 0 }, {}, { true }, { _, _, _, _ -> error("T1182 native effect") },
                        MutableStateFlow(null), scope, {}, seed)
                }
            }
        } finally { owner.cancel(); runBlocking { owner.join() } }
    }
}
