package com.blent.connection

import com.blent.MediaLocalConsent
import kotlinx.coroutines.*
import kotlinx.coroutines.flow.collect
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.junit.runners.Parameterized

@RunWith(Parameterized::class)
class ConnectionStaleControlFailureTest(
    resourceName: String,
    private val throwing: Boolean,
    private val afterNativeExit: Boolean,
) {
    private val resource = ConnectionManagement.Resource.valueOf(resourceName)
    companion object {
        @JvmStatic @Parameterized.Parameters(name = "{0}-throw={1}-retired={2}")
        fun cases(): Collection<Array<Any>> = listOf(ConnectionManagement.Resource.CAMERA,
            ConnectionManagement.Resource.MICROPHONE, ConnectionManagement.Resource.SPEAKERS).flatMap { resource ->
            listOf(false, true).flatMap { throwing ->
                listOf(false, true).map { retired -> arrayOf<Any>(resource.name, throwing, retired) }
            }
        }
    }
    private class Worker(val report: (ConnectionWorkerEvent) -> Unit) : ConnectionWorker {
        val released = CompletableDeferred<Unit>()
        var cancelled = 0
        var control: () -> Boolean = { true }
        override suspend fun execute() = awaitCancellation()
        override fun cancel() { cancelled++ }
        override suspend fun retire() = released.await()
        override fun command(value: ConnectionWorkerCommand) = control()
    }

    private class Rig {
        val scope = CoroutineScope(SupervisorJob() + Dispatchers.Unconfined)
        val workers = mutableListOf<Worker>()
        val binding = ConnectionBinding(scope, { _, report -> Worker(report).also(workers::add) })

        fun connect() {
            binding.connect(ConnectionWorkerSpec("T1241 original metadata owner"))
            val worker = workers.last()
            authenticate(worker)
            worker.report(ConnectionWorkerEvent.Camera(cameraRoute()))
            worker.report(ConnectionWorkerEvent.AudioRoute(audioRoute(ConnectionManagement.Resource.MICROPHONE)))
            worker.report(ConnectionWorkerEvent.AudioRoute(audioRoute(ConnectionManagement.Resource.SPEAKERS)))
        }

        fun authenticate(worker: Worker) {
            worker.report(ConnectionWorkerEvent.Peer(cameraLease().connection, ConnectionPeer.parse("cd".repeat(32)), true))
            worker.report(ConnectionWorkerEvent.State(ConnectionWorkerPhase.AUTHENTICATED))
        }

        fun action(resource: ConnectionManagement.Resource): () -> Boolean {
            val view = binding.state.value
            return when (resource) {
                ConnectionManagement.Resource.CAMERA -> {
                    val choice = checkNotNull(view.camera);
                    { binding.cameraChoice(choice, CameraConsentChoice(true, true, false)) }
                }
                ConnectionManagement.Resource.MICROPHONE -> {
                    val choice = checkNotNull(view.microphone);
                    { binding.audioChoice(choice, MediaLocalConsent(true, true, false, false)) }
                }
                else -> {
                    val choice = checkNotNull(view.speakers);
                    { binding.audioChoice(choice, MediaLocalConsent(true, true, false, false)) }
                }
            }
        }

        fun replace(original: Worker, release: Boolean) {
            selectReplacement(original, release)
            if (release) {
                assertEquals(2, workers.size)
                authenticate(workers.last())
            }
        }

        fun selectReplacement(original: Worker, release: Boolean) {
            binding.connect(ConnectionWorkerSpec("T1241 replacement"))
            if (release) original.released.complete(Unit)
        }

        suspend fun finishReplacement(original: Worker, released: Boolean) {
            if (!released) original.released.complete(Unit)
            withTimeout(2000) { while (workers.size < 2) delay(1) }
            assertEquals("T1241 stale failure discarded a queued replacement", 2, workers.size)
            authenticate(workers.last())
        }

        suspend fun close() {
            binding.disconnect()
            workers.forEach { it.released.complete(Unit) }
            scope.cancel()
            scope.coroutineContext[Job]!!.join()
        }
    }

    @Test fun t1241LateRejectedAndThrowingOriginalControlCannotStopReplacement() = runBlocking {
        val rig = Rig()
        try {
            rig.connect()
            val original = rig.workers.single()
            val originalAction = rig.action(resource)
            original.control = {
                // Reentrant metadata callback is a deterministic scheduling
                // point: the old caller observes failure after replacement
                // selection, on either side of original native retirement.
                rig.replace(original, afterNativeExit)
                if (throwing) error("T1241 delayed original failure") else false
            }
            assertFalse(originalAction())
            assertTrue("T1241 stale control failure stopped its replacement", rig.binding.state.value.active)
            rig.finishReplacement(original, afterNativeExit)
            assertTrue(rig.binding.state.value.authenticated)
            assertEquals(0, rig.workers.last().cancelled)
        } finally { rig.close() }
    }

    @Test fun t1241CleanupObserverCannotLoseReentrantReplacement() = runBlocking {
        val rig = Rig()
        try {
            rig.connect()
            val original = rig.workers.single()
            var selected = false
            rig.scope.launch(start = CoroutineStart.UNDISPATCHED) {
                rig.binding.state.collect { view ->
                    if (!view.active && !selected) {
                        selected = true
                        // Unconfined callbacks may defer completion jobs until
                        // this collector returns; only select inside callback.
                        rig.selectReplacement(original, afterNativeExit)
                    }
                }
            }
            original.control = { if (throwing) error("T1241 original failure") else false }
            assertFalse(rig.action(resource)())
            assertTrue(selected)
            rig.finishReplacement(original, afterNativeExit)
            assertTrue("T1241 cleanup observer replacement was stopped", rig.binding.state.value.active)
            assertTrue(rig.binding.state.value.authenticated)
            assertEquals(0, rig.workers.last().cancelled)
        } finally { rig.close() }
    }
}
