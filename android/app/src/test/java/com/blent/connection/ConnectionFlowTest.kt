package com.blent.connection

import org.json.JSONArray
import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [34])
class ConnectionFlowTest {
    private fun rejected(work: () -> Unit) {
        val error = runCatching(work).exceptionOrNull()
        assertTrue("T846 invalid flow accepted: $error", error is IllegalArgumentException || error is IllegalStateException)
    }
    private fun state(flow: ConnectionFlow): List<Any?> = flow.javaClass.declaredFields.map {
        it.isAccessible = true; it.get(flow)
    }
    @Test fun t846SharedCreditAndRetirementTrace() {
        val text = javaClass.classLoader!!.getResourceAsStream("connection-flow-v1.json")!!.bufferedReader().use { it.readText() }
        val steps = JSONArray(text)
        val flow = ConnectionFlow(7, 9, 1, 64)
        repeat(steps.length()) { index ->
            val step = steps.getJSONObject(index)
            val before = state(flow)
            if (step.getBoolean("ok")) apply(flow, step) else {
                rejected { apply(flow, step) }
                assertEquals("rejection changed state at $index", before, state(flow))
            }
        }
    }
    private fun apply(flow: ConnectionFlow, step: JSONObject) {
        val length = step.getInt("length")
        if (step.getString("direction") == "release") { flow.release(length); return }
        val header = ConnectionWire.Header(step.getInt("kind"), step.getInt("channel"),
            step.getLong("session"), step.getLong("generation"), step.getLong("sequence"), length, step.getInt("credit"))
        if (step.getString("direction") == "receive") flow.receive(header)
        else assertEquals(header, flow.send(header.kind, header.length, header.credit))
    }
    @Test fun t846CapacityIdentityAndChannelIsolation() {
        for (capacity in listOf(Int.MIN_VALUE, -1, 0, ConnectionWire.MAX_CREDIT + 1, Int.MAX_VALUE)) {
            rejected { ConnectionFlow(1, 1, 1, capacity) }
        }
        rejected { ConnectionFlow(0, 1, 1, 1) }
        rejected { ConnectionFlow(1, 0, 1, 1) }
        val control = ConnectionFlow(1, 1, 1, ConnectionWire.MAX_CREDIT)
        val video = ConnectionFlow(1, 1, 2, 1)
        video.send(ConnectionWire.RESET, 0, 0)
        rejected { video.send(ConnectionWire.CREDIT, 0, 1) }
        control.send(ConnectionWire.CREDIT, 0, ConnectionWire.MAX_CREDIT)
        val header = ConnectionWire.Header(ConnectionWire.DATA, 1, 1, 1, 0, 1, 0)
        control.receive(header)
        rejected { control.send(ConnectionWire.CREDIT, 0, 1) }
        control.release(1)
        control.send(ConnectionWire.CREDIT, 0, 1)
        control.receive(header.copy(kind = ConnectionWire.CLOSE, length = 0, sequence = 1))
        rejected { control.receive(header) }
        val replacement = ConnectionFlow(1, 2, 1, 1)
        rejected { replacement.receive(header) }
        rejected { replacement.release(Int.MAX_VALUE) }
    }
    @Test fun t846SequencesNeverWrap() {
        val flow = ConnectionFlow(1, 1, 1, 16)
        setCounter(flow, "sendSequence", Long.MAX_VALUE)
        val header = flow.send(ConnectionWire.CREDIT, 0, 1)
        assertEquals(Long.MAX_VALUE, header.sequence)
        val before = state(flow)
        rejected { flow.send(ConnectionWire.CREDIT, 0, 1) }
        assertEquals(before, state(flow))
        setCounter(flow, "receiveSequence", Long.MAX_VALUE)
        flow.receive(header)
        val received = state(flow)
        rejected { flow.receive(header) }
        assertEquals(received, state(flow))
    }
    @Test fun t846SeededDuplexFlowBounds() {
        val sender = ConnectionFlow(1, 1, 2, 64)
        val receiver = ConnectionFlow(1, 1, 2, 64)
        sender.receive(receiver.send(ConnectionWire.CREDIT, 0, 64))
        val random = java.util.Random(846)
        repeat(1024) {
            val size = random.nextInt(64) + 1
            val header = sender.send(ConnectionWire.DATA, size, 0)
            assertEquals(64 - size, sender.available())
            val before = state(receiver)
            rejected { receiver.receive(header.copy(generation = 2)) }
            assertEquals(before, state(receiver))
            receiver.receive(header)
            rejected { receiver.send(ConnectionWire.CREDIT, 0, size) }
            receiver.release(size)
            sender.receive(receiver.send(ConnectionWire.CREDIT, 0, size))
            assertEquals(64, sender.available())
        }
        sender.send(ConnectionWire.RESET, 0, 0)
        assertEquals(0, sender.available())
    }
    private fun setCounter(flow: ConnectionFlow, name: String, value: Long) {
        flow.javaClass.getDeclaredField(name).apply { isAccessible = true }.setLong(flow, value)
    }
}
