package com.blent

import android.util.Log
import kotlinx.coroutines.*
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.FlowCollector

/** Main-actor foreground demand for one borrowed Activity Window. Epoch cutoff
 * fences old callbacks; this does not release the framework-owned Window or prove
 * decoder retirement. Unknown flag writes retain that Window's OS/lifecycle owner. */
internal class WindowPowerBinding(
    private val write: (Boolean) -> Unit,
    private val report: (Exception) -> Unit = { Log.w("BlentWindow", "Original wake request failed", it) },
) {
    private class Original(val scope: CoroutineScope)
    private var current: Original? = null

    fun start(initial: DisplayPowerDemand, updates: Flow<DisplayPowerDemand>) {
        if (current != null) return
        val original = Original(CoroutineScope(Dispatchers.Main.immediate + SupervisorJob()))
        current = original
        apply(original, initial)
        if (current !== original) return
        original.scope.launch { observe(original, updates) }
    }
    private suspend fun observe(original: Original, updates: Flow<DisplayPowerDemand>) {
        try {
            updates.collect(object : FlowCollector<DisplayPowerDemand> {
                override suspend fun emit(value: DisplayPowerDemand) { apply(original, value) }
            })
        }
        catch (canceled: CancellationException) { if (current === original) stop(); throw canceled }
        catch (failure: Exception) { failed(original, failure) }
    }
    private fun apply(original: Original, demand: DisplayPowerDemand) {
        if (current !== original) return
        try {
            val keep = demand.power.keepScreenOn && demand.permit.permitsPower()
            if (current === original) write(keep)
        } catch (failure: Exception) { failed(original, failure) }
    }
    private fun failed(original: Original, failure: Exception) {
        if (current === original) stop()
        notify(failure)
    }
    fun stop() {
        val original = current ?: return
        current = null
        original.scope.cancel()
        try { write(false) } catch (failure: Exception) { notify(failure) }
    }
    private fun notify(failure: Exception) {
        try { report(failure) }
        catch (report: Exception) { if (report !== failure) failure.addSuppressed(report) }
    }
}
