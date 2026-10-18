package com.blent

import android.app.Application

/** Process-lifetime composition root. Original power handoffs and quarantined
 * native work keep this sponsor after their foreground producer disappears.
 * Private media processes never initialize this lazy main-service owner. */
open class BlentApplication : Application() {
    internal val streamingPower: StreamingPowerRuntime by lazy { StreamingPowerRuntime(this) }
}
