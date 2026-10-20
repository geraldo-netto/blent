package com.blent.connection

import com.blent.LifecycleRun
import com.blent.ResourceRetirement
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.runBlocking
import org.junit.Assert.*
import org.junit.Test

class ConnectionAccessMediaTest {
    @Test fun t1167LateNativeClaimsCannotAddressReplacementAfterOriginalRelayRetires() = runBlocking {
        val commands = listOf(ConnectionCameraConsumerCommands,
            ConnectionAudioConsumerCommands(ConnectionManagement.Resource.MICROPHONE),
            ConnectionAudioConsumerCommands(ConnectionManagement.Resource.SPEAKERS))
        for (kind in commands) verifyOriginalAuthority(kind)
    }

    private suspend fun verifyOriginalAuthority(commands: ConnectionMediaConsumerCommands) {
        val access = ConnectionAccess()
        val originalSends = mutableListOf<ConnectionWorkerCommand>()
        val replacementSends = mutableListOf<ConnectionWorkerCommand>()
        val lease = cameraLease().copy(key = cameraLease().key.copy(resource = commands.resource))
        val original = ConnectionMediaConsumerOwner(lease, commands,
            { originalSends.add(it); true }, Dispatchers.Unconfined)
        val replacement = ConnectionMediaConsumerOwner(lease, commands,
            { replacementSends.add(it); true }, Dispatchers.Unconfined)
        val oldClaim = checkNotNull(original.retain())
        val newClaim = checkNotNull(replacement.retain())
        val guarded = access.guard(ConnectionUse.CONNECTION, object : LifecycleRun {
            override suspend fun execute() {}
            override fun cancel() { original.invalidate() }
            override suspend fun retire() { original.retire() }
        })
        try {
            guarded.execute(); guarded.cancel()
            assertFalse(oldClaim.current())
            assertEquals(ConnectionUse.CONNECTION, access.current())
            // Domain trace: private-worker proof is the delegate's premise;
            // a separately owned native consumer has not supplied its receipt.
            guarded.retire()
            assertNull(access.current())
            access.acquire(ConnectionUse.SAVED_TRUST).use {
                oldClaim.retired(ResourceRetirement.RETIRED)
                oldClaim.retired(ResourceRetirement.NOT_STARTED)
                assertFalse(original.stop()); assertNull(original.retain())
                assertTrue(originalSends.isEmpty()); assertTrue(replacementSends.isEmpty())
                assertTrue(newClaim.current())
                assertEquals(ConnectionUse.SAVED_TRUST, access.current())
            }
        } finally {
            guarded.retire(); original.retire(); replacement.retire()
            oldClaim.retired(ResourceRetirement.RETIRED)
            newClaim.retired(ResourceRetirement.RETIRED)
        }
        assertNull(access.current())
    }
}
