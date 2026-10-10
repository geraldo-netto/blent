package com.blent.connection

import java.nio.ByteBuffer
import java.nio.ByteOrder

/** T850 portable management schema. Parsing never supplies identity or permission.
 * Local endpoint addresses/ports and OS handles cannot appear in this contract. */
internal object ConnectionManagement {
    const val HEADER_BYTES = 28
    const val MAX_RECORD_BYTES = HEADER_BYTES + 256
    enum class Resource(val wire: Int) { DISPLAY(1), CAMERA(2), MICROPHONE(3), SPEAKERS(4) }
    enum class Outcome { ADMITTED, UNAVAILABLE, PERMISSION_REQUIRED, PERMISSION_DENIED, BUSY, REVOKED, FAILED }
    enum class Reason { REQUESTED, CONNECTION_LOST, WORKER_FAILED, TIMED_OUT, PERMISSION_REVOKED }
    data class Key(val resource: Resource, val request: Long, val generation: Long)
    /** Owned credential bytes; default object identity string never formats them. */
    class Credential(bytes: ByteArray) {
        private val owned = bytes.also { require(it.size == 32) }.copyOf()
        fun bytes(): ByteArray = owned.copyOf()
    }
    sealed interface Profile
    data object Display : Profile
    data class Camera(val width: Int, val height: Int, val fps: Int, val bitrate: Int,
        val minBitrate: Int, val freshnessMs: Int, val lens: Int,
        val background: Boolean, val adaptive: Boolean) : Profile
    data class Audio(val processing: Int, val bufferMs: Int, val background: Boolean) : Profile
    sealed interface Message
    data class Capabilities(val mask: Int) : Message
    data class Offer(val key: Key, val credential: Credential, val profile: Profile) : Message
    data class Prepared(val key: Key) : Message
    data class Commit(val key: Key) : Message
    data class ResultMessage(val key: Key, val outcome: Outcome) : Message
    data class Revoke(val key: Key, val reason: Reason) : Message
    internal data class Header(val operation: Int, val resource: Int, val request: Long, val generation: Long, val length: Int)

    private fun resource(value: Int): Resource {
        require(value in 1..4)
        return Resource.entries[value - 1]
    }
    private fun key(message: Message): Key? = when (message) {
        is Capabilities -> null
        is Offer -> message.key
        is Prepared -> message.key
        is Commit -> message.key
        is ResultMessage -> message.key
        is Revoke -> message.key
    }
    private fun operation(message: Message): Int = when (message) {
        is Capabilities -> 0
        is Offer -> 1
        is Prepared -> 2
        is Commit -> 3
        is ResultMessage -> 4
        is Revoke -> 5
    }
    private fun offerBytes(resource: Int): Int = when (resource) {
        1 -> 32
        2 -> 45
        3, 4 -> 36
        else -> throw IllegalArgumentException("Unknown management resource")
    }
    private fun bodyBytes(operation: Int, resource: Int): Int = when (operation) {
        0, 4, 5 -> 1
        1 -> offerBytes(resource)
        2, 3 -> 0
        else -> throw IllegalArgumentException("Unknown management operation")
    }
    private fun validate(header: Header) {
        if (header.operation == 0) {
            require(header.resource == 0 && header.request == 0L && header.generation == 0L)
        } else {
            resource(header.resource)
            require(header.request > 0 && header.generation > 0)
        }
        require(header.length == bodyBytes(header.operation, header.resource))
    }
    internal fun header(bytes: ByteArray): Header {
        require(bytes.size == HEADER_BYTES)
        val buffer = ByteBuffer.wrap(bytes).order(ByteOrder.BIG_ENDIAN)
        require(buffer.int == 0x424c4d47 && buffer.get().toInt() == 1)
        val operation = buffer.get().toInt() and 255
        val resource = buffer.get().toInt() and 255
        require(buffer.get().toInt() == 0)
        return Header(operation, resource, buffer.long, buffer.long, buffer.int).also(::validate)
    }
    fun encode(message: Message): ByteArray {
        val body = ManagementProfiles.encode(message)
        val key = key(message)
        val header = Header(operation(message), key?.resource?.wire ?: 0,
            key?.request ?: 0, key?.generation ?: 0, body.size).also(::validate)
        return ByteBuffer.allocate(HEADER_BYTES + body.size).order(ByteOrder.BIG_ENDIAN).apply {
            putInt(0x424c4d47); put(1); put(header.operation.toByte()); put(header.resource.toByte()); put(0)
            putLong(header.request); putLong(header.generation); putInt(body.size); put(body)
        }.array()
    }
    fun decode(bytes: ByteArray): Message {
        require(bytes.size in HEADER_BYTES..MAX_RECORD_BYTES)
        val header = header(bytes.copyOfRange(0, HEADER_BYTES))
        require(bytes.size == HEADER_BYTES + header.length)
        val body = bytes.copyOfRange(HEADER_BYTES, bytes.size)
        if (header.operation == 0) {
            val mask = body[0].toInt() and 255
            require(mask in 0..15)
            return Capabilities(mask)
        }
        val key = Key(resource(header.resource), header.request, header.generation)
        return decodeBody(header.operation, key, body)
    }
    private fun decodeBody(operation: Int, key: Key, body: ByteArray): Message = when (operation) {
        1 -> ManagementProfiles.decode(key, body)
        2 -> Prepared(key)
        3 -> Commit(key)
        4 -> {
            val code = body[0].toInt() and 255; require(code in Outcome.entries.indices)
            ResultMessage(key, Outcome.entries[code])
        }
        5 -> {
            val code = body[0].toInt() and 255; require(code in Reason.entries.indices)
            Revoke(key, Reason.entries[code])
        }
        else -> throw IllegalArgumentException("Unknown management operation")
    }
}
