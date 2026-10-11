package com.blent.connection

import com.blent.MediaLocalConsent
import org.junit.Assert.*
import org.junit.Test

private class PermissionRuntimeRun(val base: AudioRuntimeRun) : ConnectionRun by base,
    LocalAudioControl by base, LocalAudioPermissionControl {
    var prompt: AudioPermissionRequest? = null
    val replies = mutableListOf<ConnectionWorkerCommand.AudioPermission>()
    override fun pendingAudio() = prompt
    override fun audioPermissionResult(prompt: AudioPermissionRequest, granted: Boolean, choice: MediaLocalConsent, now: Long): Boolean {
        if (this.prompt !== prompt) return false
        this.prompt = null; replies.add(ConnectionWorkerCommand.AudioPermission(prompt.lease, granted, choice)); return true
    }
}

class ConnectionAudioPermissionRuntimeTest {
    private val lease = audioLease(ConnectionManagement.Resource.MICROPHONE)
    private val context = MediaLocalConsent(true, true, true, false)
    @Test fun t997RuntimeResolvesOnlyOriginalPendingPermissionAndRejectsForeignOrLostAuthority() {
        val setup = RuntimeSetup().apply { offer() }
        val base = AudioRuntimeRun(); val run = PermissionRuntimeRun(base)
        val runtime = ConnectionRuntimeOwner(setup, ConnectionRunFactory { run }, { 0 })
        val reply = ConnectionWorkerCommand.AudioPermission(lease, true, context)
        assertFalse(runtime.command(reply)); runtime.advance()
        assertFalse(runtime.command(reply)); run.prompt = AudioPermissionRequest(lease)
        assertFalse(runtime.command(reply.copy(lease = lease.copy(owner = 2))))
        assertTrue(runtime.command(reply)); assertEquals(listOf(reply), run.replies)
        assertFalse(runtime.command(reply))
        val random = java.util.Random(997); run.prompt = AudioPermissionRequest(lease)
        repeat(128) {
            val counter = (random.nextLong() and Long.MAX_VALUE).coerceAtLeast(2)
            assertFalse(runtime.command(reply.copy(lease = lease.copy(owner = counter))))
            assertFalse(runtime.command(reply.copy(lease = lease.copy(connection = lease.connection.copy(epoch = counter)))))
        }
        assertEquals(1, run.replies.size); assertNotNull(run.prompt)
        setup.live = false
        assertFalse(runtime.command(reply)); assertTrue(base.stopped); assertEquals(1, run.replies.size)
    }
    @Test fun t997AudioWithoutOptionalPermissionAdapterNeverClaimsSdkReply() {
        val setup = RuntimeSetup().apply { offer() }; val base = AudioRuntimeRun()
        val runtime = ConnectionRuntimeOwner(setup, ConnectionRunFactory { base }, { 0 }); runtime.advance()
        assertFalse(runtime.command(ConnectionWorkerCommand.AudioPermission(lease, false, context)))
        assertFalse(base.stopped); assertTrue(base.choices.isEmpty())
    }
}
