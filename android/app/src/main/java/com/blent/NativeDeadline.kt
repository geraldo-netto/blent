package com.blent

import kotlinx.coroutines.*
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.asStateFlow

/** Independent notification only: expiry never releases the native owner. */
internal class NativeDeadline(private val label: String, private val milliseconds: Long = 5_000) {
    init { require(milliseconds in 1..60_000) }
    private val mutableStatus = MutableStateFlow<String?>(null)
    val status = mutableStatus.asStateFlow()
    private var timer: Job? = null
    private var generation = 0L

    @Synchronized fun begin() {
        if (timer != null) return
        val revision = ++generation
        mutableStatus.value = "$label pending."
        timer = CoroutineScope(Dispatchers.Default).launch {
            delay(milliseconds)
            expired(revision)
        }
    }

    @Synchronized private fun expired(revision: Long) {
        if (revision == generation) mutableStatus.value = "$label stalled; replacement waiting."
    }

    @Synchronized fun finish() {
        generation++
        timer?.cancel()
        timer = null
        mutableStatus.value = null
    }
}
