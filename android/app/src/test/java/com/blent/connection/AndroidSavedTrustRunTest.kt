package com.blent.connection

import android.content.ContextWrapper
import kotlinx.coroutines.*
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config
import org.robolectric.util.ReflectionHelpers

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [27, 34])
class AndroidSavedTrustRunTest {
    @Test fun t1169AdapterUsesOnlyApplicationContextAndPrivateTrustOperation() = runBlocking<Unit> {
        val requests = listOf(SavedTrustRequest.List,
            SavedTrustRequest.Forget(SavedComputerSelection(savedPeer(0), 1, savedPeer(1))))
        for (request in requests) verifyAdapter(request)
    }
    private suspend fun verifyAdapter(request: SavedTrustRequest) {
        val endpoint = ConnectionWorkerEndpoint().apply { start = {
            it.emit(ConnectionWorkerEvent.SavedComputers(SavedComputersPage(null, 0, 0, 0, emptyList())))
            it.emit(ConnectionWorkerEvent.Ended(null))
        } }
        val app = ConnectionWorkerContext(endpoint)
        val activity = object : ContextWrapper(app) {
            override fun bindService(intent: android.content.Intent, connection: android.content.ServiceConnection, flags: Int): Boolean =
                error("Activity context retained")
        }
        var received = 0
        val run = androidSavedTrustRun(activity, request) { received++; assertNull(it.local) }
        assertEquals(0, app.binds)
        try { run.execute() } finally { endpoint.die(); run.retire() }
        assertEquals(1, received); assertEquals(1, app.unbinds)
        assertTrue(endpoint.codes.contains(ConnectionWorkerIpc.START_TRUST))
        assertFalse(endpoint.codes.contains(ConnectionWorkerIpc.START)); assertTrue(endpoint.controls.isEmpty())
    }
    @Test fun t1169ApplicationOwnerCachesMaintenanceAcrossViewsWithSharedConnectionExclusion() = runBlocking<Unit> {
        val previous = ReflectionHelpers.getStaticField<SavedTrustBinding?>(ConnectionOwner::class.java, "saved")
        val endpoint = ConnectionWorkerEndpoint().apply { start = {
            it.emit(ConnectionWorkerEvent.SavedComputers(SavedComputersPage(null, 0, 0, 0, emptyList())))
            it.emit(ConnectionWorkerEvent.Ended(null))
        } }
        val app = ConnectionWorkerContext(endpoint)
        ReflectionHelpers.setStaticField(ConnectionOwner::class.java, "saved", null)
        val binding = ConnectionOwner.getSaved(ContextWrapper(app))
        val client = binding.attach()
        try {
            assertSame(binding, ConnectionOwner.getSaved(app))
            val access = ReflectionHelpers.getStaticField<ConnectionAccess>(ConnectionOwner::class.java, "access")
            access.acquire(ConnectionUse.CONNECTION).use { assertFalse(client.refresh()); assertEquals(0, app.binds) }
            assertTrue(client.refresh())
            withTimeout(5000) { while (binding.state.value.phase != SavedTrustPhase.READY) delay(1) }
            assertNull(binding.state.value.computers); assertEquals(1, app.unbinds); assertNull(access.current())
        } finally {
            endpoint.die(); client.stop()
            withTimeout(5000) { while (app.binds > app.unbinds) delay(1) }
            client.close(); ReflectionHelpers.setStaticField(ConnectionOwner::class.java, "saved", previous)
        }
    }
}
