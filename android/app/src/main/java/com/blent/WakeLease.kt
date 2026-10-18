package com.blent

/** One original handle, installed before native admission. A failed effect
 * retains its cleanup sponsor; logical demand never proves retirement. */
internal class WakeLease(private val create: () -> WakeHandle) {
    private class Held(val handle: WakeHandle) {
        var active = false
        var failure: Throwable? = null
    }
    private var held: Held? = null
    private var operating = false
    val settled: Boolean @Synchronized get() = held == null && !operating

    @Synchronized fun request(wanted: Boolean) = operate {
        if (wanted) acquire() else release()
    }
    @Synchronized fun retire() = operate { release() }

    private fun acquire() {
        held?.let {
            check(it.active && it.failure == null) { "Original wake handle remains uncertain" }
            return
        }
        val original = Held(create())
        held = original
        effect(original) { original.handle.acquire(); original.active = true }
    }

    private fun release() {
        val original = held ?: return
        effect(original) { original.handle.retire() }
        held = null
    }

    private fun effect(original: Held, action: () -> Unit) {
        try { action() }
        catch (failure: Throwable) { original.failure = failure; throw failure }
    }

    private fun operate(action: () -> Unit) {
        check(!operating) { "Reentrant wake ownership operation" }
        operating = true
        try { action() }
        finally { operating = false }
    }
}
