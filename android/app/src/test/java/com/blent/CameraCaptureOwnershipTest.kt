package com.blent

import kotlinx.coroutines.*
import org.junit.Assert.*
import org.junit.Test

class CameraCaptureOwnershipTest {
    private val endpoint = CameraEndpoint("ab".repeat(32), 12345, 640, 480, 30, 1000)
    private class Claim : MediaConsumerClaim {
        var alive = true
        val proofs = mutableListOf<ResourceRetirement>()
        override fun current() = alive
        override fun retired(proof: ResourceRetirement) { proofs.add(proof) }
    }
    private class Authority : MediaConsumerAuthority {
        val claim = Claim()
        override fun retain() = claim.takeIf { it.alive }
        override fun stop(): Boolean { claim.alive = false; return true }
    }
    private class Run : LifecycleRun {
        var opened = 0
        var stopped = 0
        var retired = 0
        override suspend fun execute() { opened++ }
        override fun cancel() { stopped++ }
        override suspend fun retire() { retired++ }
    }

    @Test fun t943RepeatedExecutionCannotOverwriteNativeOwner() = runBlocking {
        val native = Run()
        var created = 0
        val capture = MediaConsumerRun(null, { true }, { created++; native })
        capture.execute()
        assertTrue("T943 repeated execute cannot replace retained native owner", runCatching { capture.execute() }.isFailure)
        assertEquals(1, created)
        capture.retire()
        assertEquals(1, native.retired)
    }

    @Test fun t943RetirementBeforeExecutionNeverConstructsNativeOwner() = runBlocking {
        var created = 0
        val native = Run()
        val unopened = MediaConsumerRun(null, { true }, { created++; native })
        unopened.retire()
        assertTrue("T943 retirement closes future execution", runCatching { unopened.execute() }.isFailure)
        assertEquals(0, created)
        assertEquals(0, native.opened)
    }

    @Test fun t943PermissionAndOriginalClaimRecheckedAcrossLazyConstruction() = runBlocking {
        for (afterFactory in listOf(false, true)) {
            val claim = Claim()
            val native = Run()
            var created = 0
            var permission = afterFactory
            val capture = MediaConsumerRun(claim, { permission }, { created++; permission = false; native })
            assertTrue(runCatching { capture.execute() }.isFailure)
            assertEquals(if (afterFactory) 1 else 0, created)
            assertEquals(0, native.opened)
            capture.retire()
            assertEquals(if (afterFactory) 1 else 0, native.retired)
        }
        for (afterFactory in listOf(false, true)) {
            val claim = Claim().apply { alive = afterFactory }
            val native = Run()
            var created = 0
            val capture = MediaConsumerRun(claim, { true }, { created++; claim.alive = false; native })
            assertTrue(runCatching { capture.execute() }.isFailure)
            assertEquals(if (afterFactory) 1 else 0, created)
            assertEquals(0, native.opened)
            capture.retire()
            assertEquals(if (afterFactory) 1 else 0, native.retired)
        }
    }

    @Test fun t943CancellationBeforeAndDuringFactoryCannotOpenNativeCapture() = runBlocking {
        val first = MediaConsumerRun(null, { true }, { error("cancelled factory") })
        first.cancel()
        assertTrue(runCatching { first.execute() }.isFailure)
        first.retire()
        val native = Run()
        lateinit var during: MediaConsumerRun
        during = MediaConsumerRun(null, { true }, { during.cancel(); native })
        assertTrue(runCatching { during.execute() }.isFailure)
        during.retire()
        assertEquals(0, native.opened)
        assertEquals(1, native.retired)
        val active = MediaConsumerRun(null, { true }, { native })
        active.execute(); active.cancel(); active.retire()
        assertEquals(1, native.opened)
        assertTrue("T943 cancel and retirement request native stop", native.stopped >= 2)
    }

    @Test fun t943SourceKeepsAdbFallbackAndRejectsEqualMetadataAsOwnerIdentity() {
        val source = CameraCaptureSource()
        assertNull(source.endpoint)
        assertFalse(source.invitation(null))
        assertTrue(source.invitation(endpoint))
        assertFalse(source.invitation(endpoint))
        val first = CameraCaptureTarget(endpoint, Authority())
        assertTrue(source.target(first))
        assertFalse(source.target(first))
        val second = CameraCaptureTarget(endpoint, Authority())
        assertTrue(source.target(second))
        assertSame(second, source.target)
        val fallback = endpoint.copy(port = 12346)
        assertFalse(source.invitation(fallback))
        assertEquals(endpoint, source.endpoint)
        assertTrue(source.target(null))
        assertEquals(fallback, source.endpoint)
        assertTrue(source.invitation(null))
        assertNull(source.endpoint)
    }

    @Test fun t943BoundedInvalidProfilesRejectBeforeAuthorityOrNativeOpening() {
        val authority = Authority()
        val invalid = listOf(endpoint.copy(token = ""), endpoint.copy(port = 0), endpoint.copy(port = 65536),
            endpoint.copy(width = 159), endpoint.copy(height = 1081), endpoint.copy(fps = 31),
            endpoint.copy(bitrate = 255), endpoint.copy(freshnessMs = 2001), endpoint.copy(minBitrate = 20001))
        for (value in invalid) assertThrows(IllegalArgumentException::class.java) { CameraCaptureTarget(value, authority) }
        val random = java.util.Random(943)
        repeat(128) {
            val value = random.nextInt()
            if (value !in 1..65535) assertThrows(IllegalArgumentException::class.java) { CameraCaptureTarget(endpoint.copy(port = value), authority) }
        }
        assertTrue(authority.claim.alive)
        assertTrue(authority.claim.proofs.isEmpty())
    }
}
