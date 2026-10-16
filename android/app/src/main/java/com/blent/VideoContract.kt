package com.blent

/** Protocol values and limits, independent of transport and native decoder ownership. */
internal object VideoWire {
    const val MIME_TYPE = "video/avc"
    const val MIME_TYPE_HEVC = "video/hevc"
    const val MAX_FRAME_SIZE = 8 * 1024 * 1024
    const val PACKET_TYPE_CONFIG = 0
    const val PACKET_TYPE_FRAME = 1
    const val PACKET_TYPE_LIVENESS = 2
    const val LIVENESS_VERSION = 1
    const val FRAME_HEADER_SIZE = 5
}

internal data class DecoderFormat(val mimeType: String, val width: Int, val height: Int, val fps: Int,
                                  val codecPrivate: ByteArray? = null, val selection: DecoderSelection? = null)

internal interface DecoderInputInfo {
    val size: Int
    val configuration: Boolean
    val sequence: Long
}

/** Immutable render metadata with a lock-free retirement check. Consumers check
 * current ownership after acquiring their own transport admission lock. */
internal class RenderAcknowledgement(
    val sequence: Int,
    val decodeMicros: Int,
    val decoder: String? = null,
    val current: () -> Boolean = { true },
)
