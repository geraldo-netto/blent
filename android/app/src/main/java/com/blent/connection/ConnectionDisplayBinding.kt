package com.blent.connection

import com.blent.DisplayEndpoint
import com.blent.DisplayRouteTarget

/** Activity-owned boundary adapter. Main-thread calls only; no Context, native
 * work or application-owned Activity reference. Initial disconnected state preserves ADB;
 * an active USB selection with no route revokes the old destination. Choice identity includes
 * process lifetime, so equal counters in a replacement still replace routes. */
internal class ConnectionDisplayBinding(private val target: DisplayRouteTarget) {
    private var selected: ConnectionDisplayChoice? = null
    private var claimed = false
    fun apply(view: ConnectionView) {
        val choice = admitted(view)
        if (!claimed && choice == null && !view.active) return
        if (claimed && selected === choice) return
        claimed = true
        selected = choice
        val route = choice?.route
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
