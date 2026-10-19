package com.blent.connection

import org.junit.Assert.*
import org.junit.Test
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicLong

class AccessoryAdmissionOwnerTest {
    private val profile = AccessoryProfile("Blent", "Blent", "1")
    private val description = AccessoryDescription("Blent", "Blent", "test", "1", null, null)
    internal class Backend : AccessoryAdmissionBackend {
        lateinit var dispatch: (() -> Unit) -> Unit
        lateinit var report: (AccessorySelection, List<AccessoryDescription>, Boolean) -> Unit
        lateinit var detach: () -> Unit
        @Volatile var requested: AccessorySelection? = null
        @Volatile var permitted = false
        @Volatile var closed = false
        @Volatile var ended = true
        @Volatile var error: Exception? = null
        var entries = emptyList<AccessoryDescription>()
        var starting: () -> Unit = {}
        var snapshotting: () -> Unit = {}
        var requesting: () -> Unit = {}
        var closing: () -> Unit = {}
        override fun start(dispatch: (() -> Unit) -> Unit,
            report: (AccessorySelection, List<AccessoryDescription>, Boolean) -> Unit, detached: () -> Unit) {
            this.dispatch = dispatch; this.report = report; detach = detached; starting()
        }
        override fun snapshot(): List<AccessoryDescription> { snapshotting(); return entries }
        override fun permitted(selection: AccessorySelection) = permitted
        override fun request(selection: AccessorySelection) { requested = selection; requesting() }
        override fun failure() = error
        override fun close() { closing(); closed = true }
        override fun retired() = closed && ended
    }
    private fun backend() = Backend().apply { entries = listOf(description) }
    private fun await(condition: () -> Boolean) {
        val end = System.nanoTime() + TimeUnit.SECONDS.toNanos(3)
        while (!condition()) { check(System.nanoTime() < end) { "Admission test timeout" }; Thread.sleep(1) }
    }
    private fun latch(value: CountDownLatch) { check(value.await(3, TimeUnit.SECONDS)) }
    private fun owner(backend: Backend, clock: AtomicLong = AtomicLong()) = AccessoryAdmissionOwner(
        backend, profile, clock = clock::get, nativeTimeoutMs = 10, permissionTimeoutMs = 100, retirementTimeoutMs = 20)
    private fun end(owner: AccessoryAdmissionOwner) { owner.stop(); await { owner.retired() }; owner.stop() }
    private fun granted(backend: Backend, owner: AccessoryAdmissionOwner): AccessorySelection {
        await { backend.requested != null }
        val selected = checkNotNull(backend.requested)
        backend.dispatch { backend.report(selected, backend.entries, true) }
        var result: AccessorySelection? = null
        await { result = owner.takeReady(); result != null }
        assertEquals(selected, result); assertTrue(owner.current(selected))
        return selected
    }
    @Test fun t889PermissionThenDetachInvalidatesBeforeWakeAndDiscardsLateResults() {
        val backend = backend(); val owner = owner(backend)
        try {
            val selected = granted(backend, owner)
            backend.detach()
            assertFalse(owner.current(selected)); assertNull(owner.takeReady())
            backend.report(selected, backend.entries, true)
            backend.dispatch { error("Stopped callback must not execute") }
            assertNull(owner.takeReady())
        } finally { end(owner) }
    }
    @Test fun t889AlreadyGrantedAdmissionAndMonotonicPermissionDeadline() {
        val clock = AtomicLong(); val backend = backend().apply { permitted = true }; val owner = owner(backend, clock)
        try {
            var selected: AccessorySelection? = null
            await { selected = owner.takeReady(); selected != null }
            assertNull(backend.requested); clock.set(10000); owner.advance()
            assertTrue(owner.current(checkNotNull(selected))); assertNull(owner.failure()); assertFalse(owner.retired())
            assertFalse(owner.terminationRequired())
        } finally { end(owner) }
    }
    @Test fun t889WrongSelectionCallbackCannotApproveOrInvalidateCurrentSelection() {
        val backend = backend(); val owner = owner(backend)
        try {
            await { backend.requested != null }; val selected = checkNotNull(backend.requested)
            backend.report(selected.copy(epoch = selected.epoch + 1), emptyList(), false)
            assertTrue(owner.current(selected)); assertNull(owner.failure()); assertNull(owner.takeReady())
            granted(backend, owner)
        } finally { end(owner) }
    }
    @Test fun t889DenialAmbiguityChangedDeviceAndLateGrantReject() {
        for (mode in 0..3) {
            val clock = AtomicLong(); val backend = backend(); val owner = owner(backend, clock)
            try {
                await { backend.requested != null }; val selected = checkNotNull(backend.requested)
                if (mode == 3) clock.set(100)
                val entries = when (mode) { 1 -> listOf(description, description); 2 -> emptyList(); else -> backend.entries }
                backend.report(selected, entries, mode != 0)
                assertFalse(owner.current(selected)); assertNotNull(owner.failure()); assertNull(owner.takeReady())
            } finally { end(owner) }
        }
    }
    @Test fun t889IndependentPollFindsNativeFailureAndPermissionTimeout() {
        for (nativeFailure in listOf(false, true)) {
            val clock = AtomicLong(); val backend = backend(); val owner = owner(backend, clock)
            try {
                await { backend.requested != null }
                if (nativeFailure) backend.error = IllegalStateException("Permission callback dispatch failed") else clock.set(100)
                owner.advance(); assertNotNull(owner.failure()); assertNull(owner.takeReady())
            } finally { end(owner) }
        }
    }
    @Test fun t889NativeStartStallTimesOutAndLateStartDoesNotEnumerate() {
        val entered = CountDownLatch(1); val release = CountDownLatch(1); val clock = AtomicLong()
        var snapshots = 0
        val backend = backend().apply { starting = { entered.countDown(); latch(release) }; snapshotting = { snapshots++ } }
        val owner = owner(backend, clock)
        try {
            latch(entered); clock.set(10); owner.advance(); await { backend.closed }
            assertNotNull(owner.failure()); assertFalse(owner.retired())
            clock.set(30); owner.advance(); assertTrue(owner.terminationRequired())
        } finally { release.countDown(); end(owner) }
        assertEquals("T889: cancelled native start cannot begin another native operation", 0, snapshots)
    }
    @Test fun t889NativeRequestHasShorterDeadlineThanHumanApproval() = nativeDeadline(true)
    @Test fun t889NativeCallbackHasShorterDeadlineThanHumanApproval() = nativeDeadline(false)
    private fun nativeDeadline(duringRequest: Boolean) {
        run {
            val entered = CountDownLatch(1); val release = CountDownLatch(1); val clock = AtomicLong()
            val backend = backend()
            val held = { entered.countDown(); latch(release) }
            if (duringRequest) backend.requesting = held
            val owner = owner(backend, clock)
            try {
                await { backend.requested != null }
                if (!duringRequest) backend.dispatch(held)
                latch(entered); clock.set(10); owner.advance()
                assertNotNull("T889: native call budget must not inherit human approval timeout", owner.failure())
                assertFalse(owner.current(checkNotNull(backend.requested)))
                assertFalse(owner.retired())
            } finally { release.countDown(); end(owner) }
        }
    }
    @Test fun t889PermissionWaitKeepsOriginalConsentDeadlineAfterCallbacks() {
        val clock = AtomicLong(); val backend = backend(); val owner = owner(backend, clock)
        try {
            await { backend.requested != null }
            val finished = CountDownLatch(1)
            clock.set(80); backend.dispatch { finished.countDown() }; latch(finished)
            clock.set(100); owner.advance()
            assertNotNull(owner.failure()); assertNull(owner.takeReady())
        } finally { end(owner) }
    }
    @Test fun t889BoundedStalePermissionMutationsNeverAdmitWork() {
        val random = java.util.Random(889); val backend = backend(); val owner = owner(backend)
        try {
            await { backend.requested != null }; val selected = checkNotNull(backend.requested)
            repeat(256) {
                val stale = selected.copy(epoch = 2 + random.nextInt(Int.MAX_VALUE - 2).toLong())
                backend.report(stale, if (random.nextBoolean()) backend.entries else emptyList(), random.nextBoolean())
                assertNull(owner.takeReady()); assertTrue(owner.current(selected)); assertNull(owner.failure())
            }
            granted(backend, owner)
        } finally { end(owner) }
    }
    @Test fun t889DuplicatePermissionCallbackNeverPublishesSecondAdmission() {
        val backend = backend(); val owner = owner(backend)
        try {
            val selected = granted(backend, owner)
            backend.report(selected, backend.entries, true)
            assertNull("T889: one worker admission per USB selection", owner.takeReady())
        } finally { end(owner) }
    }
    @Test fun t889NativeCloseFailureAndUnretiredBackendRetainOwnership() {
        val entered = CountDownLatch(1); val release = CountDownLatch(1); val clock = AtomicLong()
        val backend = backend().apply { closing = { entered.countDown(); latch(release) }; ended = false }
        val owner = owner(backend, clock)
        try {
            granted(backend, owner); owner.stop(); latch(entered); assertFalse(owner.retired())
            clock.set(20); owner.advance(); assertTrue(owner.terminationRequired())
        } finally { release.countDown() }
        await { backend.closed }; assertFalse(owner.retired()); backend.ended = true; end(owner)
        val failed = backend().apply { closing = { error("close failed") } }; val broken = owner(failed)
        broken.stop(); await { broken.failure() != null }; assertTrue(broken.terminationRequired()); assertFalse(broken.retired())
    }
    @Test fun t889CallbackBacklogAndCallbackExceptionStopAdmission() {
        val entered = CountDownLatch(1); val release = CountDownLatch(1)
        val backend = backend().apply { requesting = { entered.countDown(); latch(release) } }; val owner = owner(backend)
        try {
            latch(entered); repeat(32) { backend.dispatch {} }; backend.dispatch {}
            assertNotNull(owner.failure()); assertFalse(owner.current(checkNotNull(backend.requested)))
        } finally { release.countDown(); end(owner) }
        val other = backend(); val broken = owner(other)
        try { await { other.requested != null }; other.dispatch { error("callback failure") }; await { broken.failure() != null } }
        finally { end(broken) }
    }
    @Test fun t889NativeExceptionsAtStartSnapshotPermissionRequestAreRetained() {
        for (mode in 0..2) {
            val backend = backend().apply {
                if (mode == 0) starting = { error("start") }
                if (mode == 1) snapshotting = { error("snapshot") }
                if (mode == 2) requesting = { error("request") }
            }
            val owner = owner(backend)
            try { await { owner.failure() != null }; assertNull(owner.takeReady()) } finally { end(owner) }
        }
    }
    @Test fun t889InvalidClockDoesNotResetCleanupOrRestoreAdmission() {
        for (value in listOf(-1L, Long.MAX_VALUE, 1L)) {
            val clock = AtomicLong(2); val backend = backend(); val owner = owner(backend, clock)
            try {
                await { backend.requested != null }; clock.set(value); owner.advance()
                assertNotNull(owner.failure()); assertNull(owner.takeReady())
            } finally { end(owner) }
        }
    }
    @Test fun t889ThreadLaunchAndWakeFailuresRetainActualOwner() {
        for (failsAt in listOf(1, 2)) {
            var calls = 0; val backend = backend()
            val owner = AccessoryAdmissionOwner(backend, profile, clock = { 0 }, threads = { name, body ->
                if (++calls == failsAt) error("thread factory")
                Thread(body, name).apply { isDaemon = true }
            })
            assertNotNull(owner.failure())
            if (failsAt == 1) { assertFalse(owner.retired()); assertFalse(backend.closed) } else end(owner)
        }
        val backend = backend(); val owner = AccessoryAdmissionOwner(backend, profile, clock = { 0 }, wake = { error("wake") })
        assertNotNull(owner.failure()); end(owner)
    }
    @Test fun t889InvalidCapacityDeadlineAndClockBoundsRejectBeforeThreads() {
        for (value in listOf(Long.MIN_VALUE, -1, 0, 60001, Long.MAX_VALUE)) {
            assertThrows(IllegalArgumentException::class.java) { AccessoryAdmissionOwner(backend(), profile, clock = { 0 }, nativeTimeoutMs = value) }
            assertThrows(IllegalArgumentException::class.java) { AccessoryAdmissionOwner(backend(), profile, clock = { 0 }, retirementTimeoutMs = value) }
        }
        for (value in listOf(Long.MIN_VALUE, -1, 0, 600001, Long.MAX_VALUE)) {
            assertThrows(IllegalArgumentException::class.java) { AccessoryAdmissionOwner(backend(), profile, clock = { 0 }, permissionTimeoutMs = value) }
        }
        for (value in listOf(-1L, Long.MAX_VALUE)) {
            assertThrows(IllegalArgumentException::class.java) { AccessoryAdmissionOwner(backend(), profile, clock = { value }) }
        }
        for (value in listOf(0, 1025, Int.MAX_VALUE)) {
            assertThrows(IllegalArgumentException::class.java) { AccessoryAdmissionOwner(backend(), profile, capacity = value, clock = { 0 }) }
        }
    }
}
