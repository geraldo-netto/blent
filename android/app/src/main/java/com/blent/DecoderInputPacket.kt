package com.blent

/** One complete access unit. Borrowed for a synchronous call; an asynchronous
 * consumer must detach bytes before returning and bound its retained storage. */
internal data class DecoderInput(
    val data: ByteArray, val offset: Int, override val size: Int,
    override val configuration: Boolean, override val sequence: Long,
) : DecoderInputInfo

/** Optional asynchronous input owner. Install before native configure; close
 * admission on cancellation before retiring the original native codec. Closing
 * is idempotent: cutoff and final teardown may each close the same consumer. */
internal interface DecoderInputConsumer {
    fun install()
    fun offer(input: DecoderInput): Boolean
    fun close()
}

internal interface DecoderInputEvents {
    fun queued()
    fun output(sequence: Int)
    fun failed(error: Exception)
}
