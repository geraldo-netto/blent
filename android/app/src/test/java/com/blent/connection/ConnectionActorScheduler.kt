package com.blent.connection

import java.util.concurrent.ArrayBlockingQueue
import java.util.concurrent.atomic.AtomicBoolean
import kotlin.coroutines.CoroutineContext
import kotlin.coroutines.resume
import kotlinx.coroutines.*

/** Test-only retained virtual timers. No automatic wall-clock progress or native
 * resources. Scheduled and dispatched work each have independent capacity 64. */
@OptIn(InternalCoroutinesApi::class)
internal class ConnectionActorScheduler : CoroutineDispatcher(), Delay {
    private class Timer(val at: Long, private val work: Runnable) {
        private val pending = AtomicBoolean(true)
        fun cancel() { pending.set(false) }
        fun fire() { if (pending.compareAndSet(true, false)) work.run() }
    }
    private val ready = ArrayBlockingQueue<Runnable>(64)
    private val timers = ArrayList<Timer>(64)
    @Volatile var now = 0L
        private set
    override fun dispatch(context: CoroutineContext, block: Runnable) { check(ready.offer(block)) { "T1096 dispatch capacity" } }
    private fun after(timeMillis: Long, block: Runnable): DisposableHandle {
        require(timeMillis >= 0 && now <= Long.MAX_VALUE - timeMillis)
        val timer = synchronized(timers) {
            check(timers.size < 64) { "T1096 timer capacity" }
            Timer(now + timeMillis, block).also { timers.add(it) }
        }
        return DisposableHandle { timer.cancel(); synchronized(timers) { timers.remove(timer) }; Unit }
    }
    override fun invokeOnTimeout(timeMillis: Long, block: Runnable, context: CoroutineContext): DisposableHandle = after(timeMillis, block)
    override fun scheduleResumeAfterDelay(timeMillis: Long, continuation: CancellableContinuation<Unit>) {
        val timer = after(timeMillis, Runnable { continuation.resume(Unit) })
        continuation.invokeOnCancellation { timer.dispose() }
    }
    fun runCurrent() {
        repeat(10000) {
            val next = ready.poll() ?: return
            next.run()
        }
        error("T1096 bounded actor work did not quiesce")
    }
    fun queueUntil(time: Long) {
        require(time >= now); now = time
        val due = synchronized(timers) { timers.filter { it.at <= now } }
        for (timer in due) {
            check(ready.offer(Runnable { timer.fire() })) { "T1096 due timer dispatch capacity" }
            synchronized(timers) { timers.remove(timer) }
        }
    }
    fun advanceTo(time: Long) { queueUntil(time); runCurrent() }
    fun empty(): Boolean = ready.isEmpty() && synchronized(timers) { timers.isEmpty() }
    fun nextTimerAt(): Long? = synchronized(timers) { timers.minOfOrNull { it.at } }
}
