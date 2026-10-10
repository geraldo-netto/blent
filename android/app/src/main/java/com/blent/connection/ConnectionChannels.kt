package com.blent.connection

import com.blent.connection.ConnectionManagement.Resource
import java.util.Collections

/** Fixed media mapping. Management is never a local media endpoint. */
internal object ConnectionChannels {
    private val display = Collections.unmodifiableList(listOf(ConnectionWire.CONTROL, ConnectionWire.DISPLAY))
    private val camera = listOf(ConnectionWire.CAMERA)
    private val microphone = listOf(ConnectionWire.MICROPHONE)
    private val speakers = listOf(ConnectionWire.SPEAKER)

    fun members(resource: Resource): List<Int> = when (resource) {
        Resource.DISPLAY -> display
        Resource.CAMERA -> camera
        Resource.MICROPHONE -> microphone
        Resource.SPEAKERS -> speakers
    }
    fun belongs(resource: Resource, channel: Int): Boolean = members(resource).contains(channel)
    fun resource(channel: Int): Resource = when (channel) {
        ConnectionWire.CONTROL, ConnectionWire.DISPLAY -> Resource.DISPLAY
        ConnectionWire.CAMERA -> Resource.CAMERA
        ConnectionWire.MICROPHONE -> Resource.MICROPHONE
        ConnectionWire.SPEAKER -> Resource.SPEAKERS
        else -> throw IllegalArgumentException("Not a media channel")
    }
}

/** Exact consumer-output loan. Only its owner may complete it after native
 * write and flush; transfer does not imply resource authority or retirement. */
internal class ConnectionChannelRead internal constructor(val bytes: ByteArray, val offset: Int, val length: Int)
