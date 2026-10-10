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
    private var binding: ConnectionBinding? = null

    @Synchronized fun get(context: Context): ConnectionBinding {
        val app = context.applicationContext
        return binding ?: ConnectionBinding(CoroutineScope(SupervisorJob() + Dispatchers.Default),
            { spec, report -> androidUsbConnection({ app }, spec, report) }).also { binding = it }
    }
}
