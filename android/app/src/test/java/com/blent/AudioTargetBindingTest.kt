package com.blent

import kotlinx.coroutines.*
import kotlinx.coroutines.flow.MutableStateFlow
import org.junit.Assert.*
import org.junit.Test

class AudioTargetBindingTest {
    private fun construct(scope: CoroutineScope, direction: Int, initial: AudioPreferences = AudioPreferences()) =
        AudioBinding(initial, {}, { true }, { _, _, _ -> error("T983 must not open native audio") },
            MutableStateFlow(null), scope, {}, direction, {})

    @Test fun t983InvalidInitialDirectionAndPreferencesRejectBeforeNativeOwnership() {
        val scope = CoroutineScope(SupervisorJob() + Dispatchers.Unconfined)
        try {
            for (direction in listOf(Int.MIN_VALUE, -1, 0, 3, 255, Int.MAX_VALUE)) {
                assertThrows("T983 invalid direction=$direction", IllegalArgumentException::class.java) { construct(scope, direction) }
            }
            for (direction in 1..2) {
                assertThrows(IllegalArgumentException::class.java) { construct(scope, direction, AudioPreferences(processing = 3)) }
                assertThrows(IllegalArgumentException::class.java) { construct(scope, direction, AudioPreferences(gain = -1)) }
                assertThrows(IllegalArgumentException::class.java) { construct(scope, direction, AudioPreferences(gain = if (direction == 1) 201 else 101)) }
            }
        } finally { scope.cancel() }
    }

    private class Authority : MediaConsumerAuthority {
        var available = true
        var stops = 0
        var failure: Exception? = null
        var retainHook: () -> Unit = {}
        val claims = mutableListOf<Claim>()
        inner class Claim : MediaConsumerClaim {
            val proofs = mutableListOf<ResourceRetirement>()
            override fun current() = available
            override fun retired(proof: ResourceRetirement) { proofs.add(proof) }
        }
        override fun retain(): MediaConsumerClaim? {
            retainHook()
            return if (available) Claim().also { claims.add(it) } else null
        }
        override fun stop(): Boolean { stops++; available = false; failure?.let { throw it }; return true }
    }
    private class Worker(val endpoint: AudioEndpoint, val preferences: AudioPreferences,
        val report: (String) -> Unit) : LifecycleRun {
        val release = CompletableDeferred<Unit>()
        var started = 0
        var retired = 0
        var failedExit = false
        var failure: Exception? = null
        override suspend fun execute() {
            started++
            if (failedExit) error("T983 isolated worker died")
            awaitCancellation()
        }
        override fun cancel() {}
        override suspend fun retire() { release.await(); failure?.let { throw it }; retired++ }
    }
    private class Rig(val direction: Int = 2) : AutoCloseable {
        val scope = CoroutineScope(SupervisorJob() + Dispatchers.Unconfined)
        val adb = MutableStateFlow<AudioEndpoint?>(null)
        val workers = mutableListOf<Worker>()
        val background = mutableListOf<String?>()
        var permission = true
        var prompts = 0
        var creation: (Worker) -> Unit = {}
        val endpoint = AudioEndpoint("ab".repeat(32), 12345, direction, 1, 40, false)
        val binding = AudioBinding(AudioPreferences(), { prompts++ }, { permission }, { endpoint, settings, report ->
            Worker(endpoint, settings, report).also { workers.add(it); creation(it) }
        }, adb, scope, { background.add(it) }, direction, {})
        fun target(authority: Authority, value: AudioEndpoint = endpoint) = AudioConsumerTarget(value, authority)
        override fun close() {
            workers.forEach { it.release.complete(Unit) }
            binding.shutdown(); scope.cancel()
        }
    }

    @Test fun t983ExternalTargetBeforeForegroundRunsOnceAndBackgroundRemainsOriginal() {
        Rig().use { rig ->
            rig.binding.configure(AudioPreferences(background = true, processing = 2, gain = 50, speakerLowLatency = true))
            val authority = Authority(); val target = rig.target(authority, rig.endpoint.copy(background = true))
            rig.binding.useTarget(target); assertTrue(rig.workers.isEmpty()); assertTrue(authority.available)
            rig.binding.start(); rig.binding.start(); rig.binding.useTarget(target)
            val worker = rig.workers.single()
            assertEquals(1, worker.started); assertEquals(2, worker.endpoint.processing)
            assertEquals(rig.binding.preferences, worker.preferences); assertTrue(worker.endpoint.background)
            assertEquals(1, authority.claims.size)
            val service = rig.background.last(); assertTrue(rig.binding.ownsBackground(service))
            rig.binding.stop(); assertTrue(rig.binding.sharing); assertTrue(authority.available)
            rig.binding.start(); assertEquals(1, rig.workers.size)
            rig.binding.backgroundStopped("other-original-service"); assertTrue(rig.binding.sharing)
            rig.binding.backgroundStopped(service); assertFalse(rig.binding.sharing)
            assertFalse(authority.available); assertTrue(authority.claims.single().proofs.isEmpty())
            worker.release.complete(Unit)
            assertEquals(listOf(ResourceRetirement.RETIRED), authority.claims.single().proofs)
            rig.binding.start(); rig.binding.useTarget(target); assertEquals(1, rig.workers.size)
        }
    }

    @Test fun t983EqualMetadataReplacementRetainsOldClaimsUntilNativeCleanupAndAdbCannotReplaceUsb() {
        Rig().use { rig ->
            rig.binding.start()
            val first = Authority(); val second = Authority()
            rig.binding.useTarget(rig.target(first)); val old = rig.workers.single()
            rig.binding.useTarget(rig.target(second)); assertFalse(first.available)
            assertTrue(first.claims.single().proofs.isEmpty()); assertEquals(1, rig.workers.size)
            rig.adb.value = rig.endpoint.copy(port = 12346)
            assertNull(rig.adb.value); assertEquals(1, rig.workers.size); assertEquals(0, rig.prompts)
            old.report("T983 stale status"); assertFalse(rig.binding.status.contains("stale"))
            old.release.complete(Unit); assertEquals(2, rig.workers.size)
            assertEquals(listOf(ResourceRetirement.RETIRED), first.claims.single().proofs)
            assertEquals(rig.endpoint, rig.workers.last().endpoint)
            val active = rig.workers.last(); rig.binding.useTarget(null)
            assertFalse(second.available); assertTrue(second.claims.single().proofs.isEmpty())
            active.release.complete(Unit); assertEquals(listOf(ResourceRetirement.RETIRED), second.claims.single().proofs)
            assertEquals(2, rig.workers.size); rig.binding.start(); assertEquals(2, rig.workers.size)
            rig.adb.value = rig.endpoint.copy(port = 12347); assertEquals(3, rig.workers.size)
            assertEquals(12347, rig.workers.last().endpoint.port)
        }
    }

    @Test fun t983EveryQueuedOriginalTargetGetsRetirementOrNoStartProofAndStopCannotReviveIt() {
        Rig().use { rig ->
            rig.binding.start(); val first = Authority(); val pending = Authority(); val last = Authority()
            rig.binding.useTarget(rig.target(first)); val old = rig.workers.single()
            rig.binding.useTarget(rig.target(pending)); rig.binding.useTarget(rig.target(last))
            assertFalse(pending.available)
            assertEquals(listOf(ResourceRetirement.NOT_STARTED), pending.claims.single().proofs)
            assertTrue(first.claims.single().proofs.isEmpty()); assertTrue(last.claims.single().proofs.isEmpty())
            assertEquals(1, rig.workers.size)
            old.release.complete(Unit); assertEquals(2, rig.workers.size)
            rig.binding.stopSharing(); assertFalse(last.available)
            val current = rig.workers.last(); current.release.complete(Unit)
            assertEquals(listOf(ResourceRetirement.RETIRED), last.claims.single().proofs)
            rig.binding.stop(); rig.binding.start(); rig.binding.accept()
            assertEquals(2, rig.workers.size)
        }
    }

    @Test fun t983ExternalPermissionLossOrRefusedTargetNeverLaunchesLegacySdkOrNativeFactory() {
        for (direction in 1..2) Rig(direction).use { rig ->
            rig.permission = false; rig.binding.start(); val denied = Authority()
            rig.binding.useTarget(rig.target(denied)); assertEquals(0, rig.prompts)
            assertFalse(denied.available); assertTrue(denied.claims.isEmpty()); assertTrue(rig.workers.isEmpty())
            rig.permission = true; val refused = Authority().apply { available = false }
            rig.binding.useTarget(rig.target(refused)); assertTrue(refused.claims.isEmpty()); assertTrue(rig.workers.isEmpty())
        }
    }

    @Test fun t983PermissionAndOriginalClaimAreCheckedAcrossLazyFactoryAndCreatedWorkersStayOwned() {
        Rig(1).use { rig ->
            rig.binding.start(); val owner = Authority()
            rig.creation = { rig.permission = false }
            rig.binding.useTarget(rig.target(owner)); val unopened = rig.workers.single()
            assertEquals(0, unopened.started); assertTrue(owner.claims.single().proofs.isEmpty())
            assertFalse(rig.binding.sharing); assertFalse(owner.available)
            unopened.release.complete(Unit); assertEquals(1, unopened.retired)
            assertEquals(listOf(ResourceRetirement.RETIRED), owner.claims.single().proofs)
        }
        Rig().use { rig ->
            rig.binding.start(); val owner = Authority()
            rig.creation = { owner.available = false }
            rig.binding.useTarget(rig.target(owner)); val unopened = rig.workers.single()
            assertEquals(0, unopened.started); unopened.release.complete(Unit)
            assertEquals(1, unopened.retired)
            assertEquals(listOf(ResourceRetirement.RETIRED), owner.claims.single().proofs)
        }
    }

    @Test fun t983LateRevocationBeforeQueuedNativeFactoryReleasesOnlyAfterNoNativeCanRun() {
        Rig().use { rig ->
            rig.binding.start(); val old = Authority(); rig.binding.useTarget(rig.target(old))
            val original = rig.workers.single(); val next = Authority(); rig.binding.useTarget(rig.target(next))
            next.available = false; original.release.complete(Unit)
            assertEquals(1, rig.workers.size)
            assertEquals(listOf(ResourceRetirement.RETIRED), next.claims.single().proofs)
        }
        Rig(1).use { rig ->
            rig.binding.start(); val old = Authority(); rig.binding.useTarget(rig.target(old))
            val original = rig.workers.single(); val next = Authority(); rig.binding.useTarget(rig.target(next))
            rig.permission = false; original.release.complete(Unit)
            assertEquals(1, rig.workers.size); assertEquals(0, rig.prompts)
            assertEquals(listOf(ResourceRetirement.RETIRED), next.claims.single().proofs)
        }
    }

    @Test fun t983FailedNativeCleanupQuarantinesOriginalProofButOtherDirectionProgresses() {
        Rig(1).use { microphone -> Rig(2).use { speaker ->
            microphone.binding.start(); speaker.binding.start()
            val owner = Authority(); microphone.binding.useTarget(microphone.target(owner))
            val failed = microphone.workers.single().apply { failure = IllegalStateException("T983 native exit failed") }
            microphone.binding.stopSharing(); failed.release.complete(Unit)
            assertTrue(owner.claims.single().proofs.isEmpty())
            val blocked = Authority(); microphone.binding.useTarget(microphone.target(blocked))
            assertTrue(blocked.claims.isEmpty()); assertFalse(blocked.available)
            assertEquals(1, microphone.workers.size); assertTrue(microphone.binding.status.contains("native exit failed"))
            val independent = Authority(); speaker.binding.useTarget(speaker.target(independent))
            assertEquals(1, speaker.workers.single().started); assertTrue(speaker.binding.sharing)
            assertTrue(independent.available)
        } }
    }

    @Test fun t983FailedAdmissionAfterRetainReportsNoStartWithoutLosingOriginalFailedWorker() {
        Rig().use { rig ->
            rig.binding.start(); val old = Authority(); rig.binding.useTarget(rig.target(old))
            val failed = rig.workers.single().apply { failure = IllegalStateException("T983 held original failure") }
            val next = Authority().apply { retainHook = { failed.release.complete(Unit) } }
            rig.binding.useTarget(rig.target(next))
            assertEquals(1, rig.workers.size); assertTrue(old.claims.single().proofs.isEmpty())
            assertEquals(listOf(ResourceRetirement.NOT_STARTED), next.claims.single().proofs)
            assertFalse(next.available); assertFalse(rig.binding.sharing)
        }
    }

    @Test fun t983ExplicitTargetDiscardsStaleLegacyPermissionAndThrowingControlStillRetiresNative() {
        Rig(1).use { rig ->
            rig.binding.start(); rig.permission = false; rig.adb.value = rig.endpoint
            val oldPermission = rig.binding.permissionPending; assertNotNull(oldPermission)
            rig.permission = true; val target = Authority(); rig.binding.useTarget(rig.target(target))
            assertNull(rig.binding.pending); rig.binding.permissionResult(false, oldPermission)
            assertTrue(rig.binding.sharing); assertEquals(1, rig.workers.size)
            target.failure = IllegalStateException("T983 control failed")
            rig.binding.useTarget(null); assertFalse(rig.binding.sharing)
            rig.workers.single().release.complete(Unit)
            assertEquals(listOf(ResourceRetirement.RETIRED), target.claims.single().proofs)
        }
    }

    @Test fun t983WorkerDeathRetainsClaimUntilActualNativeRetirement() {
        Rig().use { rig ->
            rig.binding.start(); rig.creation = { it.failedExit = true }
            val authority = Authority(); rig.binding.useTarget(rig.target(authority))
            assertFalse(rig.binding.sharing); assertFalse(authority.available)
            assertTrue(rig.binding.status.contains("isolated worker died"))
            assertTrue(authority.claims.single().proofs.isEmpty())
            rig.workers.single().release.complete(Unit)
            assertEquals(listOf(ResourceRetirement.RETIRED), authority.claims.single().proofs)
        }
    }

    @Test fun t983WrongDirectionRejectsBeforeDisturbingCurrentTargetAndProfilesStayBounded() {
        Rig().use { rig ->
            rig.binding.start(); val authority = Authority(); rig.binding.useTarget(rig.target(authority))
            val foreign = Authority(); val wrong = rig.endpoint.copy(direction = 1)
            assertThrows(IllegalArgumentException::class.java) { rig.binding.useTarget(rig.target(foreign, wrong)) }
            assertTrue(authority.available); assertTrue(foreign.claims.isEmpty()); assertEquals(1, rig.workers.size)
            assertEquals(rig.endpoint, rig.workers.single().endpoint)
            val random = java.util.Random(983)
            repeat(256) {
                val port = random.nextInt()
                if (port !in 1..65535) assertThrows(IllegalArgumentException::class.java) { rig.target(foreign, rig.endpoint.copy(port = port)) }
            }
            for (value in listOf(0, -1, 65536, Int.MIN_VALUE, Int.MAX_VALUE)) {
                assertThrows(IllegalArgumentException::class.java) { rig.target(foreign, rig.endpoint.copy(port = value)) }
            }
            invalidProfiles(rig, foreign)
        }
    }
    private fun invalidProfiles(rig: Rig, authority: Authority) {
        for (buffer in 0..220) {
            assertEquals(buffer in 20..200 && buffer % 10 == 0,
                runCatching { rig.target(authority, rig.endpoint.copy(bufferMs = buffer)) }.isSuccess)
        }
        for (processing in -1..3) {
            assertEquals(processing in 1..2, runCatching { rig.target(authority, rig.endpoint.copy(processing = processing)) }.isSuccess)
        }
        for (length in 0..128) {
            assertEquals(length == 64, runCatching { rig.target(authority, rig.endpoint.copy(token = "a".repeat(length))) }.isSuccess)
        }
        assertTrue(authority.claims.isEmpty())
        val source = AudioConsumerPublication(Any(), rig.target(authority))
        assertSame(authority, source.target.authority)
    }

    @Test fun t983BackgroundDeniedBeforeNativeStartReleasesClaimAndSettingsStopOriginalTarget() {
        Rig().use { rig ->
            rig.binding.start(); val denied = Authority()
            rig.binding.useTarget(rig.target(denied, rig.endpoint.copy(background = true)))
            assertTrue(denied.claims.isEmpty()); assertFalse(denied.available); assertTrue(rig.workers.isEmpty())
            val accepted = Authority(); rig.binding.useTarget(rig.target(accepted))
            val original = rig.workers.single(); rig.binding.configure(AudioPreferences(gain = 50))
            assertFalse(accepted.available); assertFalse(rig.binding.sharing)
            assertTrue(accepted.claims.single().proofs.isEmpty()); original.release.complete(Unit)
            assertEquals(listOf(ResourceRetirement.RETIRED), accepted.claims.single().proofs)
            rig.binding.accept(); assertEquals(1, rig.workers.size)
        }
    }

}
