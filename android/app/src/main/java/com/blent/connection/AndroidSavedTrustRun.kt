package com.blent.connection

import android.content.Context
import com.blent.LifecycleRun

/** Lazy adapter to the existing exclusive private process. Only application
 * context is retained; Ended and original retirement stay in the Binder owner. */
internal fun androidSavedTrustRun(
    context: Context,
    request: SavedTrustRequest,
    report: (SavedComputersPage) -> Unit,
): LifecycleRun = AndroidConnectionWorkerRun(context.applicationContext, ConnectionProcessLaunch.Trust(request),
    { event ->
        check(event is ConnectionWorkerEvent.SavedComputers) { "Unexpected saved-computer event" }
        report(event.page)
    })
