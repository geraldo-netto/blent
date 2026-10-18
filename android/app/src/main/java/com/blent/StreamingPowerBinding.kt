package com.blent

import android.content.Context
import kotlinx.coroutines.*
import kotlinx.coroutines.flow.Flow

/** Original Activity producer; queued handoff/native cleanup remain owned by
 * the application runtime. An old producer cannot stop a replacement. */
internal class StreamingPowerBinding(
    context: Context,
    private val runtime: StreamingPowerRuntime = StreamingPowerRuntime.forApp(context),
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
        observer.launch {
            var previous = initial
            requests.collect { request ->
                if (active && owner === original && request != previous) {
                    previous = request
                    if (!runtime.publish(original, request.power, request.permit)) stop()
                }
            }
        }
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
