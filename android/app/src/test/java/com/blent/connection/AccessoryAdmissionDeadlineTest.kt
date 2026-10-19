package com.blent.connection

import java.io.IOException
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicLong
import org.junit.Assert.*
import org.junit.Test

private class AdmissionDeadlineRig(
    val backend: AccessoryAdmissionOwnerTest.Backend = AccessoryAdmissionOwnerTest.Backend(),
    initialTime: Long = 0,
    permissionMs: Long = 100,
) : AutoCloseable {
    val clock = AtomicLong(initialTime)
    val releases = ArrayList<CountDownLatch>()
    val owner: AccessoryAdmissionOwner
    init {
        backend.entries = listOf(AccessoryDescription("Blent", "Blent", "T1094", "1", null, null))
        owner = AccessoryAdmissionOwner(backend, AccessoryProfile("Blent", "Blent", "1"), clock = clock::get,
            nativeTimeoutMs = 10, permissionTimeoutMs = permissionMs, retirementTimeoutMs = 20)
    }
    fun budget(expected: ConnectionDeadline) { setupAwait { owner.nextDeadline() == expected } }
    fun grant(): AccessorySelection {
        setupAwait { backend.requested != null }
        val selected = checkNotNull(backend.requested)
        backend.dispatch { backend.report(selected, backend.entries, true) }
        var ready: AccessorySelection? = null
        setupAwait { ready = owner.takeReady(); ready != null }
        assertEquals(selected, ready)
        return selected
    }
    override fun close() {
        owner.stop(); releases.forEach { it.countDown() }
        setupAwait { owner.retired() }
    }
}

class AccessoryAdmissionDeadlineTest {
    private fun await(value: CountDownLatch) { check(value.await(3, TimeUnit.SECONDS)) }

    @Test fun t1094NativeStartAndRequestKeepShorterBudgetThanPermissionWait() {
        val start = CountDownLatch(1); val releaseStart = CountDownLatch(1)
        val request = CountDownLatch(1); val releaseRequest = CountDownLatch(1)
        val backend = AccessoryAdmissionOwnerTest.Backend().apply {
            starting = { start.countDown(); await(releaseStart) }
            requesting = { request.countDown(); await(releaseRequest) }
        }
        AdmissionDeadlineRig(backend).use { rig ->
            rig.releases.add(releaseStart); rig.releases.add(releaseRequest)
            await(start); assertEquals(ConnectionDeadline.at(10), rig.owner.nextDeadline())
            releaseStart.countDown(); await(request)
            rig.clock.set(5)
            repeat(128) { assertEquals(ConnectionDeadline.at(10), rig.owner.nextDeadline()) }
            releaseRequest.countDown(); rig.budget(ConnectionDeadline.at(100))
            rig.grant(); rig.budget(ConnectionDeadline.at(Long.MAX_VALUE))
            assertFalse(rig.owner.retired())
        }
    }

    @Test fun t1094CallbackNativeBudgetAndPermissionDeadlineDoNotResetEachOther() {
        AdmissionDeadlineRig().use { rig ->
            rig.budget(ConnectionDeadline.at(100))
            val entered = CountDownLatch(1); val release = CountDownLatch(1)
            rig.releases.add(release); rig.clock.set(80)
            rig.backend.dispatch { entered.countDown(); await(release) }
            await(entered); assertEquals(ConnectionDeadline.at(90), rig.owner.nextDeadline())
            release.countDown(); rig.budget(ConnectionDeadline.at(100))
            rig.clock.set(100); rig.owner.advance()
            assertNotNull(rig.owner.failure()); assertNull(rig.owner.takeReady())
        }
    }

    @Test fun t1094StoppedAdmissionAndLateGrantCannotRetireBlockedOriginalCloser() {
        val entered = CountDownLatch(1); val release = CountDownLatch(1)
        val backend = AccessoryAdmissionOwnerTest.Backend().apply { closing = { entered.countDown(); await(release) } }
        AdmissionDeadlineRig(backend).use { rig ->
            rig.releases.add(release)
            val selected = rig.grant(); rig.owner.stop(); await(entered)
            assertEquals(ConnectionDeadline.at(20), rig.owner.nextDeadline())
            rig.clock.set(20); rig.owner.advance()
            assertEquals(ConnectionDeadline.IDLE, rig.owner.nextDeadline())
            assertTrue(rig.owner.terminationRequired()); assertFalse(rig.owner.retired())
            rig.backend.report(selected, rig.backend.entries, true)
            assertNull(rig.owner.takeReady()); assertFalse(rig.owner.current(selected))
            release.countDown(); setupAwait { rig.owner.retired() }
            assertEquals(ConnectionDeadline.IDLE, rig.owner.nextDeadline())
        }
    }

    @Test fun t1094ActualAdmissionRetirementRemovesUnexpiredBudget() {
        AdmissionDeadlineRig().use { rig ->
            rig.owner.stop(); setupAwait { rig.owner.retired() }
            assertEquals(ConnectionDeadline.IDLE, rig.owner.nextDeadline())
        }
    }

    @Test fun t1094CloseFailureKeepsUnconfirmedNativeOwnershipDespiteIdleMetadata() {
        val backend = AccessoryAdmissionOwnerTest.Backend().apply { closing = { throw IOException("T1094 close") } }
        AdmissionDeadlineRig(backend).use { rig ->
            try {
                rig.grant(); rig.owner.stop(); setupAwait { rig.owner.terminationRequired() }
                assertEquals(ConnectionDeadline.IDLE, rig.owner.nextDeadline())
                assertFalse(rig.owner.retired())
            } finally {
                backend.closing = {}; backend.close(); setupAwait { rig.owner.retired() }
            }
        }
    }

    @Test fun t1094MaximumHumanPermissionDeadlineIsNeverAnIdleSentinel() {
        AdmissionDeadlineRig(initialTime = Long.MAX_VALUE - 600000, permissionMs = 600000).use { rig ->
            rig.budget(ConnectionDeadline.at(Long.MAX_VALUE))
            assertNull(rig.owner.takeReady()); assertFalse(rig.owner.retired())
        }
    }
}
