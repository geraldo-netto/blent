package com.blent

import kotlinx.coroutines.*
import java.util.concurrent.atomic.AtomicBoolean

/** One query generation until confirmed process retirement. Caller deadlines
 * never authorize a replacement around an unconfirmed owner. */
internal class WorkerQueries(
    private val create: (MediaTask.Metadata, (WorkerEvent) -> Unit) -> LifecycleRun,
    private val deadlineMillis: Long = 5000,
    private val scope: CoroutineScope = CoroutineScope(SupervisorJob() + Dispatchers.Default),
) {
    private val occupied = AtomicBoolean()
    val busy get() = occupied.get()
    @OptIn(ExperimentalCoroutinesApi::class)
    suspend fun query(task: MediaTask.Metadata): String? {
        currentCoroutineContext().ensureActive()
        if (!occupied.compareAndSet(false, true)) return null
        val result = CompletableDeferred<String?>()
        val job = scope.launch(start = CoroutineStart.ATOMIC) { execute(task, result) }
        try { return withTimeoutOrNull(deadlineMillis) { result.await() } }
        finally { job.cancel() }
    }
    private suspend fun execute(task: MediaTask.Metadata, result: CompletableDeferred<String?>) {
        var run: LifecycleRun? = null
        var value: String? = null
        try {
            run = create(task) { if (it is WorkerEvent.Result) value = it.text }
            run.execute()
            result.complete(value)
        } catch (_: Exception) { result.complete(null) }
        finally {
            withContext(NonCancellable) {
                val cancellation = runCatching { run?.cancel() }
                val retirement = runCatching { run?.retire() }
                if (cancellation.isSuccess && retirement.isSuccess) occupied.set(false)
            }
        }
    }
}
