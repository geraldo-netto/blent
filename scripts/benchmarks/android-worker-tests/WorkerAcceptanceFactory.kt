package com.blent

import android.content.Context
import android.os.Process
import android.view.Surface
import kotlinx.coroutines.*
import java.util.concurrent.CountDownLatch

/** Only the generated disposable app substitutes this native factory. */
internal object WorkerAcceptanceFactory {
    fun create(context: Context, task: MediaTask, surface: Surface?, report: (WorkerEvent) -> Unit): LifecycleRun {
        if (task is MediaTask.Metadata) return LocalMediaRuns.create(context, task, surface, report)
        val mode = when (task) {
            is MediaTask.Audio -> task.endpoint.bufferMs
            is MediaTask.Camera -> 40
            is MediaTask.Display -> { check(surface!!.isValid); 40 }
            else -> error("unsupported fixture")
        }
        return AcceptanceRun(mode, report)
    }
}


/** Test-native operation owns only its bounded mode and original report callback. */
private class AcceptanceRun(private val mode: Int, private val report: (WorkerEvent) -> Unit) : LifecycleRun {
    override suspend fun execute() {
        report(WorkerEvent.Status("pid=${Process.myPid()}"))
        when (mode) {
            60 -> CountDownLatch(1).await()
            80 -> return
            100 -> { delay(50); Process.killProcess(Process.myPid()); awaitCancellation() }
            else -> while (currentCoroutineContext().isActive) { report(WorkerEvent.Status("heartbeat")); delay(20) }
        }
    }
    override fun cancel() {}
    override suspend fun retire() { if (mode == 80) CountDownLatch(1).await() }
}
