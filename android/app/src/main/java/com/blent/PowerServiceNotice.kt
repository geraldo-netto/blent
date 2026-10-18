package com.blent

import android.content.Context
import android.content.Intent
import android.os.Binder
import android.os.Bundle

/** Local Binder identity survives queueing without counter reuse. A proxy from
 * a dead/foreign process cannot become this process's original capability. */
internal class PowerServiceNotice(val runtime: StreamingPowerRuntime, val notice: DisplayPowerNotice) : Binder() {
    private val original get() = notice.original.power ?: StreamingPower(display = DisplayWakeState.BACKGROUND)
    fun intent(context: Context): Intent = original.intent(context).putExtra("power_notice", Bundle().apply { putBinder("original", this@PowerServiceNotice) })
    fun matches(intent: Intent): Boolean = StreamingPower.read(intent) == original
    companion object {
        fun read(intent: Intent?): PowerServiceNotice? = intent?.getBundleExtra("power_notice")?.getBinder("original") as? PowerServiceNotice
    }
}
