package com.blent

import android.graphics.SurfaceTexture
import android.view.Surface
import android.view.SurfaceHolder
import android.view.SurfaceView
import kotlinx.coroutines.*
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.RuntimeEnvironment
import org.robolectric.annotation.Config
import java.util.ArrayDeque
import java.util.concurrent.atomic.AtomicLong
import kotlin.coroutines.CoroutineContext
import kotlin.coroutines.Continuation
import kotlin.coroutines.intrinsics.suspendCoroutineUninterceptedOrReturn

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [27, 34])
@OptIn(InternalCoroutinesApi::class)
class SurfaceWakeupTest {
    /** Timers never advance: readiness must schedule work itself. */
    private class Scheduler : CoroutineDispatcher(), Delay {
        private val queue = ArrayDeque<Runnable>()
        val delays = mutableListOf<Long>()
        override fun dispatch(context: CoroutineContext, block: Runnable) { queue.add(block) }
        override fun scheduleResumeAfterDelay(timeMillis: Long, continuation: CancellableContinuation<Unit>) {
            delays.add(timeMillis)
        }
        fun runCurrent() {
            var remaining = 1000
            while (queue.isNotEmpty()) { check(remaining-- > 0); queue.removeFirst().run() }
        }
    }
    private class Fixture : AutoCloseable {
        val scheduler = Scheduler()
        val scope = CoroutineScope(SupervisorJob() + scheduler)
        val receiver = VideoReceiver { error("T799 must not connect") }
        private val surface = Surface(SurfaceTexture(1))
        private val view = object : SurfaceView(RuntimeEnvironment.getApplication()) {
            override fun getHolder(): SurfaceHolder = object : SurfaceHolder by super.getHolder() {
                override fun getSurface(): Surface = this@Fixture.surface
            }
        }
        init { running(7) }
        fun running(generation: Long) {
            VideoReceiver::class.java.getDeclaredField("isRunning").apply { isAccessible = true }.set(receiver, true)
            val field = VideoReceiver::class.java.getDeclaredField("sessionGeneration").apply { isAccessible = true }
            (field.get(receiver) as AtomicLong).set(generation)
        }
        fun ready() { assertTrue(surface.isValid); receiver.setSurface(view) }
        fun waitFor(generation: Long = 7): Job = scope.launch {
            suspendCoroutineUninterceptedOrReturn<Unit> { continuation ->
                val method = VideoReceiver::class.java.getDeclaredMethod("awaitSurface", Long::class.javaPrimitiveType, Continuation::class.java)
                    .apply { isAccessible = true }
                try { method.invoke(receiver, generation, continuation) }
                catch (error: java.lang.reflect.InvocationTargetException) { throw error.targetException }
            }
        }
        override fun close() {
            receiver.stop(); scope.cancel(); scheduler.runCurrent(); surface.release()
        }
    }

    @Test fun t799_readinessAfterWaitWakesWithoutAdvancingAPollTimer() {
        Fixture().use { fixture ->
            val waiter = fixture.waitFor()
            fixture.scheduler.runCurrent()
            assertFalse(waiter.isCompleted)
            fixture.ready()
            fixture.scheduler.runCurrent()
            assertTrue("T799 ready Surface still waits for 200ms timer", waiter.isCompleted)
            assertTrue(fixture.scheduler.delays.isEmpty())
        }
    }

    @Test fun t799_readyBeforeWaitAndRecreationHaveNoLostNotification() {
        Fixture().use { fixture ->
            fixture.ready()
            val first = fixture.waitFor()
            fixture.scheduler.runCurrent()
            assertTrue(first.isCompleted)
            fixture.receiver.onSurfaceDestroyed()
            fixture.running(8)
            val second = fixture.waitFor(8)
            fixture.scheduler.runCurrent()
            assertFalse(second.isCompleted)
            fixture.ready()
            fixture.receiver.onSurfaceDestroyed() // Invalidate before dispatched wakeup.
            fixture.scheduler.runCurrent()
            assertFalse("T799 stale readiness survived destruction", second.isCompleted)
            fixture.ready()
            fixture.scheduler.runCurrent()
            assertTrue(second.isCompleted)
            assertTrue(fixture.scheduler.delays.isEmpty())
        }
    }

    @Test fun t799_stopAndInvalidGenerationsFinishWithoutTimerOrSurface() {
        Fixture().use { fixture ->
            for (generation in listOf(Long.MIN_VALUE, -1, 0, 6, 8, Long.MAX_VALUE)) {
                val retired = fixture.waitFor(generation)
                fixture.scheduler.runCurrent()
                assertTrue(retired.isCompleted)
            }
            val current = fixture.waitFor()
            fixture.scheduler.runCurrent()
            assertFalse(current.isCompleted)
            fixture.receiver.stop()
            fixture.scheduler.runCurrent()
            assertTrue("T799 Stop left a Surface waiter parked", current.isCompleted)
            assertTrue(fixture.scheduler.delays.isEmpty())
        }
    }
}
