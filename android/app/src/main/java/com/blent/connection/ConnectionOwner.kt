package com.blent.connection

import android.content.Context
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob

/** Application-process owner, independent of Activity lifetime. Only application
 * context is retained. Explicit disconnect/revoke retires the link; UI recreation
 * or a permission dialog cannot discard a still-owned native worker. Individual
 * media owners retain their separate foreground/background and consent policies. */
internal object ConnectionOwner {
    private val access = ConnectionAccess()
    private var binding: ConnectionBinding? = null
    private var saved: SavedTrustBinding? = null

    @Synchronized fun get(context: Context): ConnectionBinding {
        val app = context.applicationContext
        return binding ?: ConnectionBinding(CoroutineScope(SupervisorJob() + Dispatchers.Default),
            { spec, report -> androidUsbConnection({ app }, spec, report) }, access = access).also { binding = it }
    }

    @Synchronized fun getSaved(context: Context): SavedTrustBinding {
        val app = context.applicationContext
        return saved ?: SavedTrustBinding(CoroutineScope(SupervisorJob() + Dispatchers.Default),
            access, SavedTrustRunFactory { request, report -> androidSavedTrustRun(app, request, report) }).also { saved = it }
    }
}
