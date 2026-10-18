package com.blent

import android.content.Context
import kotlinx.coroutines.*
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.FlowCollector

/** Original Activity producer; queued handoff/native cleanup remain owned by
 * the application runtime. An old producer cannot stop a replacement. */
internal class StreamingPowerBinding(
    context: Context,
    private val runtime: StreamingPowerRuntime = StreamingPowerRuntime.forApp(context),
    private val report: (Exception) -> Unit = { android.util.Log.w("BlentPower", "Original policy observer failed", it) },
) {
    private var scope: CoroutineScope? = null
    private var active = false
    private var owner: DisplayPowerOwner? = null

    fun start(initial: DisplayPowerDemand, requests: Flow<DisplayPowerDemand>) {
        if (active) return
        val original = runtime.open(initial.power, initial.permit)
        if (!runtime.owns(original)) return
        owner = original
        active = true
        val observer = CoroutineScope(Dispatchers.Main.immediate + SupervisorJob())
        scope = observer
        observer.launch { observe(original, initial, requests) }
    }
    private suspend fun observe(original: DisplayPowerOwner, initial: DisplayPowerDemand, requests: Flow<DisplayPowerDemand>) {
        try { publishUpdates(original, initial, requests) }
        catch (failure: Exception) {
            endOriginal(original, failure)
            if (failure is CancellationException) throw failure
            notify(failure)
        }
    }
    private suspend fun publishUpdates(original: DisplayPowerOwner, initial: DisplayPowerDemand, requests: Flow<DisplayPowerDemand>) {
        var previous = initial
        requests.collect(object : FlowCollector<DisplayPowerDemand> {
            override suspend fun emit(value: DisplayPowerDemand) {
                if (active && owner === original && value != previous) {
                    previous = value
                    if (!runtime.publish(original, value.power, value.permit)) stopOriginal(original)
                }
            }
        })
    }
    private fun stopOriginal(original: DisplayPowerOwner) {
        if (owner === original) stop()
    }
    private fun endOriginal(original: DisplayPowerOwner, failure: Exception) {
        try { stopOriginal(original) }
        catch (cleanup: Exception) {
            if (cleanup !== failure) failure.addSuppressed(cleanup)
            if (failure is CancellationException) notify(cleanup)
        }
    }
    private fun notify(failure: Exception) {
        try { report(failure) }
        catch (report: Exception) { if (report !== failure) failure.addSuppressed(report) }
    }
    fun stop() {
        if (!active) return
        active = false
        scope?.cancel(); scope = null
        val original = owner
        owner = null
        if (original != null) runtime.close(original)
    }
}
