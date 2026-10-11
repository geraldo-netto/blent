package com.blent

import java.io.IOException
import kotlin.coroutines.CoroutineContext
import kotlinx.coroutines.*
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.receiveAsFlow
import org.junit.Assert.*
import org.junit.Test

class CameraCaptureObservationTest {
    private class Queued : CoroutineDispatcher() {
        val tasks = java.util.ArrayDeque<Runnable>()
        override fun dispatch(context: CoroutineContext, block: Runnable) { tasks.add(block) }
        fun drain() { while (tasks.isNotEmpty()) tasks.removeFirst().run() }
    }
    private class Rig(dispatcher: CoroutineDispatcher = Dispatchers.Unconfined) : AutoCloseable {
        val scope = CoroutineScope(SupervisorJob() + dispatcher)
        val source = Channel<MediaConsumerPublication<CameraCaptureTarget>>(1)
        val identity = Any()
        val view = MutableStateFlow(MediaConsentView<CameraAccessOffer>(CameraAccessOffer(identity, CameraLens.FRONT,
            false, false, true, object : MediaAccessAuthority {
                override fun current() = true
                override fun update(context: MediaLocalConsent) = true
                override fun permissionResult(granted: Boolean, context: MediaLocalConsent) = false
            }), true, true, false))
        val target = CameraCaptureTarget(CameraEndpoint("ab".repeat(32), 12345, 640, 480, 30, 3000),
            object : MediaConsumerAuthority {
                override fun retain() = object : MediaConsumerClaim {
                    override fun current() = true
                    override fun retired(proof: ResourceRetirement) {}
                }
                override fun stop() = true
            })
        val published = mutableListOf<CameraCaptureTarget?>()
        var reject = false
        val observation = MediaConsumerObservation<CameraAccessOffer, CameraEndpoint, CameraCaptureTarget>(source.receiveAsFlow(), view, { true }, {
            if (reject && it != null) throw IOException("native selection failed")
            published.add(it)
        }, scope, ::CameraCaptureTarget)
        suspend fun publish() { source.send(MediaConsumerPublication<CameraCaptureTarget>(identity, target)) }
        override fun close() { observation.retire(); source.close(); scope.cancel() }
    }

    @Test fun t932SourceCompletionAndFailureRevokeCaptureWithoutClearingLocalConsent() = runBlocking {
        for (failure in listOf(null, IOException("source failed"))) Rig().use { rig ->
            rig.publish()
            val selected = rig.published.last()!!
            val claim = selected.authority.retain()!!
            rig.source.close(failure)
            yield()
            assertNull(rig.published.last())
            assertFalse(claim.current())
            assertNull(selected.authority.retain())
            assertTrue(rig.view.value.enabled)
            assertTrue(rig.scope.isActive)
        }
    }

    @Test fun t932ConsentStopAndObserverRetirementInvalidateOnlyOriginalCapture() = runBlocking {
        Rig().use { rig ->
            rig.publish()
            val target = rig.published.last()!!
            val claim = target.authority.retain()!!
            rig.view.value = rig.view.value.copy(enabled = false)
            assertNull(rig.published.last()); assertFalse(claim.current())
            rig.view.value = rig.view.value.copy(enabled = true)
            val replacement = rig.published.last()!!
            assertNotSame(target, replacement)
            assertFalse(claim.current())
            val active = replacement.authority.retain()!!
            rig.observation.retire()
            assertNull(rig.published.last()); assertFalse(active.current())
            rig.publish()
            assertNull(rig.published.last())
        }
    }

    @Test fun t932NativeSelectionFailureAndParentCancellationDropCapture() = runBlocking {
        Rig().use { rig ->
            rig.reject = true
            rig.publish(); yield()
            assertNull(rig.published.last())
            assertTrue(rig.scope.isActive)
        }
        Rig().use { rig ->
            rig.publish()
            val claim = rig.published.last()!!.authority.retain()!!
            rig.scope.cancel(); yield()
            assertNull(rig.published.last()); assertFalse(claim.current())
        }
    }

    @Test fun t932RetirementBeforeQueuedPublicationCannotAcquireCapture() = runBlocking {
        val queued = Queued()
        Rig(queued).use { rig ->
            rig.publish()
            rig.observation.retire()
            queued.drain()
            assertTrue(rig.published.all { it == null })
            assertTrue(rig.view.value.enabled)
        }
    }
}
