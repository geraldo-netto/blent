package com.blent.connection

import java.nio.ByteBuffer
import java.nio.ByteOrder
import com.blent.connection.ConnectionManagement.Audio
import com.blent.connection.ConnectionManagement.Camera
import com.blent.connection.ConnectionManagement.Capabilities
import com.blent.connection.ConnectionManagement.Commit
import com.blent.connection.ConnectionManagement.Credential
import com.blent.connection.ConnectionManagement.Display
import com.blent.connection.ConnectionManagement.Key
import com.blent.connection.ConnectionManagement.Message
import com.blent.connection.ConnectionManagement.Offer
import com.blent.connection.ConnectionManagement.Prepared
import com.blent.connection.ConnectionManagement.Resource
import com.blent.connection.ConnectionManagement.ResultMessage
import com.blent.connection.ConnectionManagement.Revoke

/** Canonical profile bodies; no local transport address or implementation types. */
internal object ManagementProfiles {
    fun encode(message: Message): ByteArray = when (message) {
        is Capabilities -> {
            require(message.mask in 0..15)
            byteArrayOf(message.mask.toByte())
        }
        is Offer -> offer(message)
        is Prepared, is Commit -> ByteArray(0)
        is ResultMessage -> byteArrayOf(message.outcome.ordinal.toByte())
        is Revoke -> byteArrayOf(message.reason.ordinal.toByte())
    }
    private fun offer(message: Offer): ByteArray {
        val profile = when (val value = message.profile) {
            Display -> { require(message.key.resource == Resource.DISPLAY); ByteArray(0) }
            is Camera -> { require(message.key.resource == Resource.CAMERA); camera(value) }
            is Audio -> {
                require(message.key.resource == Resource.MICROPHONE || message.key.resource == Resource.SPEAKERS)
                audio(value)
            }
        }
        return message.credential.bytes() + profile
    }
    private fun validateCamera(value: Camera) {
        require(value.width in 160..1920 && value.width % 2 == 0)
        require(value.height in 120..1080 && value.height % 2 == 0)
        require(value.fps in 5..30 && value.lens in 0..1)
        require(value.bitrate in 256..20000 && value.minBitrate in 256..20000)
        require(value.freshnessMs in 50..2000)
    }
    private fun camera(value: Camera): ByteArray {
        validateCamera(value)
        val flags = (if (value.background) 1 else 0) or (if (value.adaptive) 2 else 0)
        return ByteBuffer.allocate(13).order(ByteOrder.BIG_ENDIAN).apply {
            putShort(value.width.toShort()); putShort(value.height.toShort()); put(value.fps.toByte())
            putShort(value.bitrate.toShort()); putShort(value.minBitrate.toShort()); putShort(value.freshnessMs.toShort())
            put(value.lens.toByte()); put(flags.toByte())
        }.array()
    }
    private fun validateAudio(value: Audio) {
        require(value.processing in 1..2)
        require(value.bufferMs in 20..200 && value.bufferMs % 10 == 0)
    }
    private fun audio(value: Audio): ByteArray {
        validateAudio(value)
        return ByteBuffer.allocate(4).order(ByteOrder.BIG_ENDIAN).apply {
            put(value.processing.toByte()); putShort(value.bufferMs.toShort()); put(if (value.background) 1.toByte() else 0.toByte())
        }.array()
    }
    fun decode(key: Key, body: ByteArray): Offer {
        val buffer = ByteBuffer.wrap(body).order(ByteOrder.BIG_ENDIAN)
        val credential = Credential(ByteArray(32).also { buffer.get(it) })
        val profile = when (key.resource) {
            Resource.DISPLAY -> Display
            Resource.CAMERA -> readCamera(buffer)
            Resource.MICROPHONE, Resource.SPEAKERS -> readAudio(buffer)
        }
        return Offer(key, credential, profile)
    }
    private fun readCamera(buffer: ByteBuffer): Camera {
        val width = buffer.short.toInt() and 65535; val height = buffer.short.toInt() and 65535
        val fps = buffer.get().toInt() and 255; val bitrate = buffer.short.toInt() and 65535
        val minimum = buffer.short.toInt() and 65535; val freshness = buffer.short.toInt() and 65535
        val lens = buffer.get().toInt() and 255; val flags = buffer.get().toInt() and 255
        require(flags in 0..3)
        return Camera(width, height, fps, bitrate, minimum, freshness, lens, flags and 1 != 0, flags and 2 != 0)
            .also(::validateCamera)
    }
    private fun readAudio(buffer: ByteBuffer): Audio {
        val processing = buffer.get().toInt() and 255; val bufferMs = buffer.short.toInt() and 65535
        val background = buffer.get().toInt() and 255; require(background in 0..1)
        return Audio(processing, bufferMs, background != 0).also(::validateAudio)
    }
}
