package com.blent.connection

import com.blent.DisplayEndpoint
import com.blent.DisplayRouteTarget

/** Activity-owned boundary adapter. Main-thread calls only; no Context, native
 * work or application-owned Activity reference. Untouched state preserves ADB.
 * Each source transition is applied once, even when both endpoints are null.
 * Metadata and repeated observations retain the same original transition identity. */
internal class ConnectionDisplayBinding(private val target: DisplayRouteTarget) {
    private var selected: Any? = null
    fun apply(view: ConnectionView) {
        val selection = view.displaySelection ?: return
        if (selected === selection) return
        selected = selection
        val route = admitted(view)?.route
        target.selectDisplay(route?.let { DisplayEndpoint(credential(it.credential), it.displayPort, it.controlPort) })
    }
    private fun admitted(view: ConnectionView): ConnectionDisplayChoice? {
        if (!view.active || !view.authenticated) return null
        val choice = view.display ?: return null
        return choice.takeIf { view.peer?.ticket == it.route.lease.connection }
    }
    private fun credential(value: ConnectionManagement.Credential): String {
        val digits = "0123456789abcdef"
        return buildString(64) {
            for (byte in value.bytes()) {
                val unsigned = byte.toInt() and 255
                append(digits[unsigned ushr 4]); append(digits[unsigned and 15])
            }
        }
    }
}
