package com.blent.connection

import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test
import java.util.Random
import com.blent.connection.ConnectionManagement.Offer
import com.blent.connection.ConnectionManagement.Camera
import com.blent.connection.ConnectionManagement.Audio
import com.blent.connection.ConnectionManagement.Display
import com.blent.connection.ConnectionManagement.Credential
import com.blent.connection.ConnectionManagement.Capabilities

@org.junit.runner.RunWith(org.robolectric.RobolectricTestRunner::class)
@org.robolectric.annotation.Config(sdk = [34])
class ConnectionManagementTest {
    private fun vectors(name: String): List<ByteArray> {
        val text = javaClass.classLoader!!.getResourceAsStream("connection-management-v1.json")!!.bufferedReader().use { it.readText() }
        val array = JSONObject(text).getJSONArray(name)
        return List(array.length()) { index -> array.getJSONObject(index).getString("hex").chunked(2).map { it.toInt(16).toByte() }.toByteArray() }
    }
    private fun rejected(work: () -> Unit) { assertThrows(Exception::class.java) { work() } }
    @Test fun t850IndependentVectorsRoundTripAndReject() {
        for (bytes in vectors("valid")) assertArrayEquals(bytes, ConnectionManagement.encode(ConnectionManagement.decode(bytes)))
        for (bytes in vectors("invalid")) rejected { ConnectionManagement.decode(bytes) }
        for (length in 0 until ConnectionManagement.HEADER_BYTES) rejected { ConnectionManagement.decode(ByteArray(length)) }
        rejected { ConnectionManagement.decode(ByteArray(ConnectionManagement.MAX_RECORD_BYTES + 1)) }
    }
    @Test fun t850EverySplitAndCoalescedRecordUsesBoundedConsumption() {
        for (bytes in vectors("valid")) {
            for (split in 0..bytes.size) {
                val parser = ManagementDecoder()
                val first = parser.feed(bytes, 0, split)
                assertEquals(split, first.consumed)
                val message = if (first.message != null) first.message else parser.feed(bytes, split, bytes.size - split).message
                assertNotNull(message); assertArrayEquals(bytes, ConnectionManagement.encode(message!!)); parser.end()
                rejected { parser.feed(ByteArray(0)) }
            }
            val parser = ManagementDecoder()
            for (i in bytes.indices) {
                val next = parser.feed(bytes, i, 1)
                assertEquals(1, next.consumed)
                assertEquals(i == bytes.lastIndex, next.message != null)
            }
            parser.end()
        }
        val valid = vectors("valid")
        val joined = valid.fold(ByteArray(0)) { total, bytes -> total + bytes }
        val parser = ManagementDecoder(); var offset = 0
        for (bytes in valid) {
            val next = parser.feed(joined, offset, joined.size - offset)
            assertEquals(bytes.size, next.consumed)
            assertArrayEquals(bytes, ConnectionManagement.encode(next.message!!)); offset += next.consumed
        }
        assertEquals(joined.size, offset); parser.end()
    }
    @Test fun t850MalformedOrTruncatedStreamCannotResume() {
        for (bytes in vectors("invalid")) {
            val parser = ManagementDecoder()
            rejected {
                var offset = 0
                while (offset < bytes.size) offset += parser.feed(bytes, offset, bytes.size - offset).consumed
                parser.end()
            }
            rejected { parser.feed(vectors("valid")[0]) }; rejected { parser.end() }
        }
        for (bytes in vectors("valid")) for (length in 1 until bytes.size) {
            val parser = ManagementDecoder(); parser.feed(bytes, 0, length)
            rejected { parser.end() }; rejected { parser.feed(bytes) }
        }
        val parser = ManagementDecoder(); val bytes = vectors("valid")[0]
        for ((offset, length) in listOf(-1 to 1, 0 to -1, Int.MAX_VALUE to 1, 1 to Int.MAX_VALUE, bytes.size to 1)) {
            rejected { parser.feed(bytes, offset, length) }
        }
        assertEquals(0, parser.feed(ByteArray(0)).consumed)
        assertArrayEquals(bytes, ConnectionManagement.encode(parser.feed(bytes).message!!))
        parser.end(); rejected { parser.end() }
    }
    @Test fun t955CameraAndAudioProfileBoundaries() {
        val valid = vectors("valid").map(ConnectionManagement::decode)
        val camera = valid.filterIsInstance<Offer>().first { it.profile is Camera }
        val profile = camera.profile as Camera
        for (value in listOf(Int.MIN_VALUE, -1, 0, 159, 160, 161, 1920, 1921, Int.MAX_VALUE)) {
            val changed = camera.copy(profile = profile.copy(width = value))
            if (value in 160..1920 && value % 2 == 0) ConnectionManagement.encode(changed) else rejected { ConnectionManagement.encode(changed) }
        }
        val audio = valid.filterIsInstance<Offer>().first { it.profile is Audio }
        for (value in listOf(-1, 0, 19, 20, 21, 40, 200, 201, Int.MAX_VALUE)) {
            val changed = audio.copy(profile = (audio.profile as Audio).copy(bufferMs = value))
            if (value in 20..200 && value % 10 == 0) ConnectionManagement.encode(changed) else rejected { ConnectionManagement.encode(changed) }
        }
    }
    @Test fun t850ProfileKeyAndCredentialBoundaries() {
        val valid = vectors("valid").map(ConnectionManagement::decode)
        val camera = valid.filterIsInstance<Offer>().first { it.profile is Camera }
        val profile = camera.profile as Camera
        val audio = valid.filterIsInstance<Offer>().first { it.profile is Audio }
        for (value in listOf(Long.MIN_VALUE, -1L, 0L)) {
            rejected { ConnectionManagement.encode(camera.copy(key = camera.key.copy(request = value))) }
            rejected { ConnectionManagement.encode(camera.copy(key = camera.key.copy(generation = value))) }
        }
        ConnectionManagement.encode(camera.copy(key = camera.key.copy(request = Long.MAX_VALUE, generation = Long.MAX_VALUE)))
        rejected { ConnectionManagement.encode(camera.copy(profile = Display)) }
        rejected { ConnectionManagement.encode(audio.copy(profile = profile)) }
        for (size in listOf(0, 1, 31, 33, 256)) rejected { Credential(ByteArray(size)) }
        val source = ByteArray(32) { it.toByte() }; val credential = Credential(source)
        source.fill(99); val exported = credential.bytes(); assertEquals(0, exported[0].toInt())
        exported.fill(99); assertEquals(0, credential.bytes()[0].toInt())
        for (mask in listOf(Int.MIN_VALUE, -1, 16, Int.MAX_VALUE)) rejected { ConnectionManagement.encode(Capabilities(mask)) }
    }
    @Test fun t850BoundedWireMutationRoundTripsOrRejects() {
        val random = Random(850); val originals = vectors("valid")
        repeat(4096) {
            val bytes = originals[random.nextInt(originals.size)].copyOf()
            bytes[random.nextInt(bytes.size)] = random.nextInt(256).toByte()
            val message = try { ConnectionManagement.decode(bytes) } catch (_: IllegalArgumentException) { return@repeat }
            assertArrayEquals(bytes, ConnectionManagement.encode(message))
        }
    }
}
