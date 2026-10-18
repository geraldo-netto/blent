package com.blent

/** Factory returns a lazy wrapper; native admission happens only in execute. */
internal fun interface PowerWorkerFactory {
    fun create(task: MediaTask.Wake, report: (WorkerEvent) -> Unit): LifecycleRun
}

/** Immutable actual receiver view, separate from producer intent and handles. */
internal class PowerWorkerView(val request: DisplayPowerRequest, val task: MediaTask.Wake?, val stamp: Any)
