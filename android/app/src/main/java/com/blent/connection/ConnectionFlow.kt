package com.blent.connection

/** Created by authenticated control, never by an untrusted incoming header.
 * Single connection owner serializes calls. Reserve bounded writer capacity
 * before send; any subsequent transport failure retires this generation. */
internal class ConnectionFlow(
    private val session: Long, private val generation: Long,
    private val channel: Int, private val capacity: Int,
) {
    private var sendCredit = 0
    private var receiveCredit = 0
    private var received = 0
    private var sendSequence = 0L
    private var receiveSequence = 0L
    private var sendExhausted = false
    private var receiveExhausted = false
    private var closed = false
    private var localTerminal = false
    private var peerTerminal = false

    init {
        require(capacity in 1..ConnectionWire.MAX_CREDIT)
        ConnectionWire.validate(ConnectionWire.Header(ConnectionWire.CLOSE, channel, session, generation, 0, 0, 0))
    }

    fun available(): Int = if (closed || sendExhausted) 0 else sendCredit

    fun send(kind: Int, length: Int, credit: Int): ConnectionWire.Header {
        check(!closed && !sendExhausted)
        val header = ConnectionWire.Header(kind, channel, session, generation, sendSequence, length, credit)
        ConnectionWire.validate(header)
        applySend(header)
        if (sendSequence == Long.MAX_VALUE) sendExhausted = true else sendSequence++
        return header
    }

    private fun applySend(header: ConnectionWire.Header) {
        when (header.kind) {
            ConnectionWire.DATA -> {
                check(header.length <= sendCredit)
                sendCredit -= header.length
            }
            ConnectionWire.CREDIT -> {
                check(header.credit <= capacity - receiveCredit - received)
                receiveCredit += header.credit
            }
            else -> { closed = true; localTerminal = true }
        }
    }

    fun receive(header: ConnectionWire.Header) {
        check(!closed && !receiveExhausted)
        validateReceive(header)
        applyReceive(header)
        if (receiveSequence == Long.MAX_VALUE) receiveExhausted = true else receiveSequence++
    }

    private fun validateReceive(header: ConnectionWire.Header) {
        ConnectionWire.validate(header)
        check(header.session == session && header.generation == generation && header.channel == channel)
        check(header.sequence == receiveSequence)
    }

    fun terminal(kind: Int): ConnectionWire.Header {
        require(kind == ConnectionWire.CLOSE || kind == ConnectionWire.RESET)
        check(!localTerminal && !sendExhausted)
        val header = ConnectionWire.Header(kind, channel, session, generation, sendSequence, 0, 0)
        ConnectionWire.validate(header)
        applySend(header)
        if (sendSequence == Long.MAX_VALUE) sendExhausted = true else sendSequence++
        return header
    }

    /** Discard authorized in-flight bytes until peer terminal acknowledgement. */
    fun drain(header: ConnectionWire.Header) {
        check(closed && !peerTerminal && !receiveExhausted)
        validateReceive(header)
        applyReceive(header)
        if (header.kind == ConnectionWire.DATA) received -= header.length
        if (receiveSequence == Long.MAX_VALUE) receiveExhausted = true else receiveSequence++
    }

    fun retired(): Boolean = localTerminal && peerTerminal

    private fun applyReceive(header: ConnectionWire.Header) {
        when (header.kind) {
            ConnectionWire.DATA -> {
                check(header.length <= receiveCredit)
                receiveCredit -= header.length
                received += header.length
            }
            ConnectionWire.CREDIT -> {
                check(header.credit <= capacity - sendCredit)
                sendCredit += header.credit
            }
            else -> { closed = true; peerTerminal = true }
        }
    }

    /** Return capacity only when the consumer has released these bytes. */
    fun release(bytes: Int) {
        check(!closed)
        require(bytes > 0 && bytes <= received)
        received -= bytes
    }
}
