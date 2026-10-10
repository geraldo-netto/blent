package com.blent.connection

import org.json.JSONArray
import org.junit.Assert.*
import org.junit.Test
import java.util.Random

@org.junit.runner.RunWith(org.robolectric.RobolectricTestRunner::class)
@org.robolectric.annotation.Config(sdk = [34])
class ConnectionWireTest {
    private fun valid() = ConnectionWire.Header(0, 1, 1, 2, 0, 16, 0)
    private fun rejected(work: () -> Unit) {
        try { work(); fail("T846 malformed envelope accepted") } catch (_: IllegalArgumentException) {}
    }

    @Test fun t846SharedConnectionVectors() {
        val text = javaClass.classLoader!!.getResourceAsStream("connection-v1.json")!!.bufferedReader().use { it.readText() }
        val vectors = JSONArray(text)
        repeat(vectors.length()) { index ->
            val vector = vectors.getJSONObject(index)
            val input = vector.getString("hex").chunked(2).map { it.toInt(16).toByte() }.toByteArray()
            if (!vector.getBoolean("valid")) { rejected { ConnectionWire.decode(input) }; return@repeat }
            val header = ConnectionWire.decode(input)
            assertEquals(vector.getLong("session"), header.session)
            assertEquals(vector.getLong("generation"), header.generation)
            assertEquals(vector.getLong("sequence"), header.sequence)
            assertEquals(vector.getInt("kind"), header.kind)
            assertEquals(vector.getInt("channel"), header.channel)
            assertEquals(vector.getInt("length"), header.length)
            assertEquals(vector.getInt("credit"), header.credit)
            val output = ByteArray(ConnectionWire.HEADER_BYTES)
            ConnectionWire.encode(header, output)
            assertArrayEquals(input, output)
        }
    }

    @Test fun t846HeaderExtentsAndAtomicRejection() {
        for (length in 0..80) {
            if (length == ConnectionWire.HEADER_BYTES) continue
            val output = ByteArray(length) { 0x55 }
            rejected { ConnectionWire.encode(valid(), output) }
            assertTrue(output.all { it == 0x55.toByte() })
            rejected { ConnectionWire.decode(output) }
        }
        val output = ByteArray(ConnectionWire.HEADER_BYTES) { 0x55 }
        rejected { ConnectionWire.encode(valid().copy(session = 0), output) }
        assertTrue(output.all { it == 0x55.toByte() })
        for (value in -128..255) {
            if (value !in 0..3) rejected { ConnectionWire.validate(valid().copy(kind = value)) }
            if (value !in 1..ConnectionWire.CHANNELS) rejected { ConnectionWire.validate(valid().copy(channel = value)) }
        }
    }

    @Test fun t846BoundedHeaderMutations() {
        val random = Random(846)
        val original = ByteArray(ConnectionWire.HEADER_BYTES)
        ConnectionWire.encode(valid(), original)
        repeat(4096) {
            val input = original.copyOf()
            input[random.nextInt(input.size)] = random.nextInt(256).toByte()
            val header = try { ConnectionWire.decode(input) } catch (_: IllegalArgumentException) { return@repeat }
            val output = ByteArray(ConnectionWire.HEADER_BYTES)
            ConnectionWire.encode(header, output)
            assertArrayEquals(input, output)
            assertTrue(header.length <= ConnectionWire.MAX_PAYLOAD && header.credit <= ConnectionWire.MAX_CREDIT)
        }
    }
}
