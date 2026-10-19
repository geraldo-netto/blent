package com.blent.connection

import kotlinx.coroutines.*
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [27, 34])
class ConfiguredConnectionTest {
    @Test fun t1099SavedSnapshotAppliesToNextStartWithoutReplacingActiveWorker() = runBlocking {
        ConnectionUiFixture().use { fixture ->
            val storage = PreferenceMemoryStorage()
            val store = ConnectionPreferenceStore(storage, Dispatchers.Unconfined)
            val configured = ConfiguredConnection(fixture.binding, store, fixture.scope)
            val original = ConnectionWorkerSpec("usb", timerMs = 1000)
            assertTrue(store.save(ConnectionPreferences.fields(original)).isSuccess)
            configured.connect()
            assertEquals(listOf(original), fixture.specifications)
            assertFalse(configured.state.value.pending)
            assertTrue(store.save(ConnectionPreferences.fields(ConnectionWorkerSpec("usb"))).isSuccess)
            configured.connect()
            assertEquals(listOf(original), fixture.specifications)
            assertEquals(0, fixture.workers.single().cancelled)
            configured.disconnect()
            fixture.workers.single().released.complete(Unit)
            configured.connect()
            assertEquals(listOf(original, ConnectionWorkerSpec("usb")), fixture.specifications)
        }
    }

    @Test fun t1099ConcurrentAdmissionRechecksCurrentConnectionAfterStorageReturns() {
        ConnectionUiFixture().use { fixture ->
            val dispatcher = ConnectionActorScheduler()
            val storage = PreferenceMemoryStorage()
            val configured = ConfiguredConnection(fixture.binding, ConnectionPreferenceStore(storage, dispatcher), fixture.scope)
            configured.connect(); configured.connect()
            assertTrue(configured.state.value.pending)
            assertEquals(0, storage.reads)
            fixture.binding.connect(ConnectionWorkerSpec("other-current-owner"))
            dispatcher.runCurrent()
            assertFalse(configured.state.value.pending)
            assertEquals(1, storage.reads)
            assertEquals(listOf(ConnectionWorkerSpec("other-current-owner")), fixture.specifications)
            assertEquals(0, fixture.workers.single().cancelled)
            assertTrue(dispatcher.empty())
        }
    }

    @Test fun t1099DisconnectAndOwnerLossFencePendingOriginalStart() {
        ConnectionUiFixture().use { fixture ->
            val dispatcher = ConnectionActorScheduler()
            val storage = PreferenceMemoryStorage()
            val scope = CoroutineScope(SupervisorJob() + Dispatchers.Unconfined)
            try {
                val configured = ConfiguredConnection(fixture.binding, ConnectionPreferenceStore(storage, dispatcher), scope)
                configured.connect(); configured.disconnect(); configured.connect()
                assertTrue(configured.state.value.pending)
                dispatcher.runCurrent()
                assertFalse(configured.state.value.pending)
                assertTrue(fixture.workers.isEmpty())
                configured.connect(); scope.cancel(); dispatcher.runCurrent()
                assertTrue(fixture.workers.isEmpty())
                assertFalse(configured.state.value.pending)
                configured.connect()
                assertFalse(configured.state.value.pending)
                assertTrue(dispatcher.empty())
            } finally { scope.cancel(); dispatcher.runCurrent() }
        }
    }

    @Test fun t1099CorruptAndUnconfirmedSettingsRemainVisibleWithoutStartingWorkers() = runBlocking {
        ConnectionUiFixture().use { fixture ->
            val storage = PreferenceMemoryStorage()
            val store = ConnectionPreferenceStore(storage, Dispatchers.Unconfined)
            val configured = ConfiguredConnection(fixture.binding, store, fixture.scope)
            storage.encoded = "broken"
            configured.connect()
            assertNotNull(configured.state.value.failure)
            assertTrue(fixture.workers.isEmpty())
            storage.accepted = false
            assertTrue(store.save(ConnectionPreferences.fields(ConnectionWorkerSpec("usb"))).isFailure)
            configured.connect()
            assertTrue(configured.state.value.failure.orEmpty().contains("not confirmed"))
            assertTrue(fixture.workers.isEmpty())
            storage.accepted = true
            assertTrue(store.save(ConnectionPreferences.fields(ConnectionWorkerSpec("usb"))).isSuccess)
            configured.connect()
            assertNull(configured.state.value.failure)
            assertEquals(1, fixture.workers.size)
        }
    }

    @Test fun t1099PresentationErrorsHaveBoundedFallbackAndNoRetainedException() {
        assertEquals("USB settings request failed.", connectionPreferenceFailure(Exception()))
        assertEquals("x".repeat(256), connectionPreferenceFailure(Exception("x".repeat(1000))))
        assertEquals("", connectionPreferenceFailure(Exception("")))
    }
}
