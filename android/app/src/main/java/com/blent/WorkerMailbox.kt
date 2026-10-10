package com.blent

import kotlinx.coroutines.channels.Channel

/** Bounded IPC staging. Admission never waits on a consumer or native owner. */
internal class WorkerMailbox {
    private val messages = Channel<String>(32)
    private var bytes = 0
    @Synchronized fun offer(text: String): Boolean {
        val size = text.length.toLong() * 2
        if (size > 512 * 1024 - bytes) return false
        bytes += size.toInt()
        if (messages.trySend(text).isSuccess) return true
        bytes -= size.toInt()
        return false
    }
    suspend fun receive(): String = messages.receive().also { consumed(it) }
    @Synchronized private fun consumed(text: String) { bytes -= text.length * 2 }
    fun close(error: Exception? = null) { messages.close(error) }
}
