package com.blent

import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.selects.select

/** Immutable boundary representations: encode once at the worker, decode once
 * at the receiver. No mutable state or native ownership is duplicated. */
internal data class WorkerMessage(val event: WorkerEvent, val text: String)

/** 32 ordered events / 512 KiB encoded text plus one
 * replaceable rendered receipt (<=512 KiB encoded text). No native ownership.
 * Close rejects admission and discards the last receipt; ordered events drain. */
internal class WorkerMailbox {
    private data class Ordered(val message: WorkerMessage, val bytes: Int)
    private val messages = Channel<Ordered>(32)
    private val rendered = Channel<WorkerMessage>(Channel.CONFLATED)
    private var bytes = 0
    private var closed = false
    @Synchronized fun offer(message: WorkerMessage): Boolean {
        if (closed || message.text.length > WorkerEventJson.LIMIT) return false
        return admit(message)
    }
    @Synchronized fun offer(event: WorkerEvent): Boolean {
        if (closed) return false
        return admit(WorkerMessage(event, WorkerEventJson.encode(event)))
    }
    private fun admit(message: WorkerMessage): Boolean {
        if (message.event is WorkerEvent.Rendered) return rendered.trySend(message).isSuccess
        val size = message.text.length.toLong() * 2
        if (size > 512 * 1024 - bytes) return false
        bytes += size.toInt()
        if (messages.trySend(Ordered(message, size.toInt())).isSuccess) return true
        bytes -= size.toInt()
        return false
    }
    suspend fun receive(): WorkerMessage = select {
        messages.onReceive { consumed(it); it.message }
        rendered.onReceive { it }
    }
    @Synchronized private fun consumed(message: Ordered) { bytes -= message.bytes }
    @Synchronized fun close(error: Exception? = null) {
        closed = true
        messages.close(error)
        rendered.cancel()
    }
}
