package com.blent

/** Serialized foreground scheduling port; post/cancel never invoke callbacks inline. False means never queued; throwing
 * can be ambiguous. Cancellation must fence this exact callback before return. */
internal interface PowerSchedule {
    fun now(): Long
    fun post(delayMillis: Long, action: Runnable): Boolean
    fun cancel(action: Runnable)
}

internal class WifiIdleTarget(
    val owner: Any,
    val deadline: Long,
    val current: () -> Boolean,
    val release: () -> Unit,
)
