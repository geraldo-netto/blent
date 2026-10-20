package com.blent

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.os.Looper
import kotlinx.coroutines.*
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.RuntimeEnvironment
import org.robolectric.Shadows.shadowOf
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [27, 34])
class CameraStopTest {
    private val original = CameraEndpoint("ab".repeat(32), 12345, 640, 480, 30, 1000, CameraLens.FRONT)
    private val replacement = original.copy(token = "cd".repeat(32), port = 23456)

    private class Worker(val report: (String) -> Unit) : LifecycleRun {
        val release = CompletableDeferred<Unit>()
        var retired = false
        override suspend fun execute() { awaitCancellation() }
        override fun cancel() {}
        override suspend fun retire() { release.await(); retired = true }
    }

    private class Rig : AutoCloseable {
        private val previous = CameraInvitations.endpoint.value
        private val owner = SupervisorJob()
        private val scope = CoroutineScope(owner + Dispatchers.Unconfined)
        val workers = mutableListOf<Worker>()
        var allowed = true
        val binding = CameraBinding({ 0 }, {}, { allowed }, { _, _, _, report ->
            Worker(report).also { workers.add(it) }
        }, CameraInvitations.endpoint, scope, {})
        init { CameraInvitations.endpoint.value = null; binding.start() }
        override fun close() {
            binding.shutdown()
            workers.forEach { it.release.complete(Unit) }
            owner.cancel()
            runBlocking { owner.join() }
            CameraInvitations.endpoint.value = previous
        }
    }

    private fun invite(value: CameraEndpoint): Intent =
        Intent(RuntimeEnvironment.getApplication(), CameraReceiver::class.java)
            .putExtra("token", value.token).putExtra("port", value.port)
            .putExtra("width", value.width).putExtra("height", value.height)
            .putExtra("fps", value.fps).putExtra("bitrate", value.bitrate)

    private fun stop(value: CameraEndpoint): Intent =
        Intent(RuntimeEnvironment.getApplication(), CameraReceiver::class.java)
            .setAction("com.blent.CAMERA_STOP").putExtra("expected_token", value.token)
            .putExtra("expected_port", value.port)

    private fun send(intent: Intent): Int {
        var result = 0
        val observer = object : BroadcastReceiver() {
            override fun onReceive(context: Context, intent: Intent) { result = resultCode }
        }
        RuntimeEnvironment.getApplication().sendOrderedBroadcast(intent, null, observer, null, 0, null, null)
        shadowOf(Looper.getMainLooper()).idle()
        return result
    }

    @Test fun t1204LenslessInvitationCannotReplaceANewerEndpoint() {
        Rig().use {
            CameraInvitations.endpoint.value = replacement
            assertEquals(0, send(invite(original)))
            assertSame(replacement, CameraInvitations.endpoint.value)
        }
    }

    @Test fun t1204ScopedStopWithdrawsOnlyOriginalCredentialAndPort() {
        Rig().use {
            CameraInvitations.endpoint.value = original
            assertEquals(1, send(stop(original)))
            assertNull(CameraInvitations.endpoint.value)
            CameraInvitations.endpoint.value = replacement
            assertEquals(1, send(stop(original)))
            assertSame(replacement, CameraInvitations.endpoint.value)
            assertEquals(1, send(stop(replacement.copy(port = original.port))))
            assertSame(replacement, CameraInvitations.endpoint.value)
            assertEquals(1, send(stop(replacement)))
            assertNull(CameraInvitations.endpoint.value)
        }
    }

    @Test fun t1204MalformedStopCannotFallThroughToStartFields() {
        Rig().use {
            CameraInvitations.endpoint.value = replacement
            val invalid = invite(original).putExtra("lens", 0).setAction("com.blent.CAMERA_STOP")
            assertEquals(0, send(invalid))
            assertSame(replacement, CameraInvitations.endpoint.value)
            assertEquals(0, send(invite(original).putExtra("lens", 0).setAction("unknown")))
            assertSame(replacement, CameraInvitations.endpoint.value)
        }
    }

    @Test fun t1204LogicalCutoffRetainsActualOriginalNativeRetirement() {
        Rig().use { rig ->
            CameraInvitations.endpoint.value = original
            val first = rig.workers.single()
            send(stop(original))
            assertNull(rig.binding.selected)
            assertFalse(first.retired)
            CameraInvitations.endpoint.value = replacement
            assertEquals(1, rig.workers.size)
            first.report("stale original callback")
            assertNotEquals("stale original callback", rig.binding.status)
            first.release.complete(Unit)
            assertTrue(first.retired)
            assertEquals(2, rig.workers.size)
            assertEquals(CameraLens.FRONT, rig.binding.selected)
        }
    }

    @Test fun t1204OldPermissionCallbackCannotGrantReplacementAfterScopedStop() {
        Rig().use { rig ->
            rig.allowed = false
            CameraInvitations.endpoint.value = original
            val old = rig.binding.permissionPending
            assertNotNull(old)
            send(stop(original))
            assertNull(rig.binding.permissionPending)
            CameraInvitations.endpoint.value = replacement
            val current = rig.binding.permissionPending
            assertNotNull(current)
            rig.allowed = true
            rig.binding.permissionResult(true, old)
            assertSame(current, rig.binding.permissionPending)
            assertTrue(rig.workers.isEmpty())
            rig.binding.permissionResult(true, current)
            assertEquals(1, rig.workers.size)
        }
    }
    @Test fun t1204SharedStopKeyAndInvalidValuesUseCurrentIndependentSchema() {
        val vector = org.json.JSONObject(javaClass.getResource("/camera-stop.json")!!.readText())
        val token = vector.getString("expected_token")
        val port = vector.getInt("expected_port")
        assertTrue(CameraStop.create(token, port)!!.matches(original.copy(port = port)))
        for (value in listOf(Int.MIN_VALUE, -1, 0, 1, 65535, 65536, Int.MAX_VALUE)) {
            assertEquals(value in 1..65535, CameraStop.create(token, value) != null)
        }
        for (length in 0..128) assertEquals(length == 64, CameraStop.create("a".repeat(length), port) != null)
        val alphabet = "0123456789abcdef".toSet()
        for (character in 0..127) {
            val candidate = "a".repeat(31) + character.toChar() + "b".repeat(32)
            assertEquals(character.toChar() in alphabet, CameraStop.create(candidate, port) != null)
        }
        for (candidate in listOf("é".repeat(32), "é".repeat(64), "界".repeat(64))) assertNull(CameraStop.create(candidate, port))
    }

    private fun expected(operation: Int, current: Int): Int = when (operation) {
        0, 1, 2 -> operation
        3, 4, 5 -> if (current == operation - 3) -1 else current
        else -> -1
    }

    private fun perform(operation: Int, keys: List<CameraEndpoint>) {
        when {
            operation < 3 -> CameraInvitations.endpoint.value = keys[operation]
            operation < 6 -> {
                val key = keys[operation - 3]
                CameraInvitations.stop(CameraStop.create(key.token, key.port)!!)
            }
            else -> CameraInvitations.endpoint.value = null
        }
    }

    @Test fun t1204BoundedCutoffTracesNeverWithdrawAnotherEndpoint() {
        val previous = CameraInvitations.endpoint.value
        val keys = listOf(original, replacement, original.copy(port = replacement.port))
        try {
            for (trace in 0 until 2401) {
                CameraInvitations.endpoint.value = null
                var model = -1
                var remaining = trace
                repeat(4) {
                    val operation = remaining % 7
                    remaining /= 7
                    model = expected(operation, model)
                    perform(operation, keys)
                    assertEquals("T1204 trace $trace", model, keys.indexOf(CameraInvitations.endpoint.value))
                }
            }
        } finally { CameraInvitations.endpoint.value = previous }
    }

    @Test fun t1204MalformedKeyAndActionCannotWithdrawAnEndpoint() {
        Rig().use {
            CameraInvitations.endpoint.value = replacement
            for (port in listOf(Int.MIN_VALUE, -1, 0, 65536, Int.MAX_VALUE)) {
                assertEquals(0, send(stop(original).putExtra("expected_port", port)))
                assertSame(replacement, CameraInvitations.endpoint.value)
            }
            val missing = stop(original).also { it.removeExtra("expected_token") }
            assertEquals(0, send(missing))
            assertNull(CameraStopIntent.read(stop(original).setAction("unknown")))
            assertSame(replacement, CameraInvitations.endpoint.value)
        }
    }

}
