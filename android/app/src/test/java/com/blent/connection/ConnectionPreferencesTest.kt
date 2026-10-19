package com.blent.connection

import org.junit.Assert.*
import org.junit.Test

private data class PreferenceRange(val field: ConnectionPreferenceField, val minimum: Long, val maximum: Long)
private fun preferenceRanges() = listOf(
    PreferenceRange(ConnectionPreferenceField.CHANNEL_BYTES, 4096, 1048576),
    PreferenceRange(ConnectionPreferenceField.QUEUED_MESSAGES, 1, 1024),
    PreferenceRange(ConnectionPreferenceField.QUANTUM, 1, 65536),
    PreferenceRange(ConnectionPreferenceField.RESOURCE_TIMEOUT, 1, 600000),
    PreferenceRange(ConnectionPreferenceField.IO_TIMEOUT, 1, 60000),
    PreferenceRange(ConnectionPreferenceField.RETIREMENT_TIMEOUT, 1, 60000),
    PreferenceRange(ConnectionPreferenceField.APPROVAL_TIMEOUT, 1, 600000),
    PreferenceRange(ConnectionPreferenceField.NATIVE_TIMEOUT, 1, 60000),
    PreferenceRange(ConnectionPreferenceField.TRUSTED_PEERS, 1, 1024),
    PreferenceRange(ConnectionPreferenceField.ACCESSORY_CAPACITY, 1, 1024),
    PreferenceRange(ConnectionPreferenceField.FALLBACK, 1, 1000),
)

class ConnectionPreferencesTest {
    @Test fun t1098DefaultsHaveIndependentBoundedDeterministicRepresentation() {
        val expected = linkedMapOf("channel_bytes" to "262144", "queued_messages" to "64", "quantum" to "65536",
            "resource_timeout_ms" to "120000", "io_timeout_ms" to "5000", "retirement_timeout_ms" to "5000",
            "approval_timeout_ms" to "120000", "native_timeout_ms" to "10000", "trusted_peers" to "64",
            "accessory_capacity" to "32", "fallback_ms" to "100")
        val original = ConnectionWorkerSpec("original-device")
        assertEquals(expected, ConnectionPreferences.fields(original))
        val encoded = "1\n" + expected.entries.joinToString("\n") { "${it.key}=${it.value}" }
        assertEquals(encoded, ConnectionPreferences.encode(original))
        assertTrue(encoded.length <= ConnectionPreferences.MAX_ENCODING)
        assertFalse(encoded.contains("original-device"))
        assertEquals(ConnectionWorkerSpec("usb"), ConnectionPreferences.decode(encoded))
        assertEquals(original, ConnectionPreferences.decode(encoded, "original-device"))
        val reversed = "1\n" + encoded.lines().drop(1).reversed().joinToString("\n")
        assertEquals(original, ConnectionPreferences.decode(reversed, "original-device"))
        val padded = encoded.replace("fallback_ms=100", "fallback_ms=0000100")
        assertEquals(encoded, ConnectionPreferences.encode(ConnectionPreferences.decode(padded)))
    }

    @Test fun t1098EveryFieldBoundaryIsPreservedWithoutChangingOtherSettings() {
        val original = ConnectionWorkerSpec("original-device")
        for (range in preferenceRanges()) for (value in listOf(range.minimum, range.maximum)) {
            val next = range.field.update(original, value.toString())
            assertEquals(value.toString(), range.field.value(next))
            val expected = ConnectionPreferences.fields(original) + (range.field.key to value.toString())
            assertEquals(expected, ConnectionPreferences.fields(next))
            assertEquals(original.device, next.device)
            assertEquals(next, ConnectionPreferences.parse(expected, original.device))
            assertEquals(next, ConnectionPreferences.decode(ConnectionPreferences.encode(next), original.device))
        }
        assertEquals(100, original.timerMs)
    }

    @Test fun t1098IndependentSimultaneousEditsReachAllTypedDomains() {
        val fields = mapOf("channel_bytes" to "4096", "queued_messages" to "8", "quantum" to "64",
            "resource_timeout_ms" to "1", "io_timeout_ms" to "1", "retirement_timeout_ms" to "60000",
            "approval_timeout_ms" to "600000", "native_timeout_ms" to "60000", "trusted_peers" to "1",
            "accessory_capacity" to "1024", "fallback_ms" to "1000")
        val expected = ConnectionWorkerSpec("usb", ConnectionSessionLimits(4096, 8, 64, 1),
            ConnectionRunLimits(1, 60000), 600000, 60000, 1, 1024, 1000)
        val actual = ConnectionPreferences.parse(fields)
        assertEquals(expected, actual)
        assertEquals(fields, ConnectionPreferences.fields(actual))
        assertEquals(expected, ConnectionPreferences.decode(ConnectionPreferences.encode(actual)))
    }

    @Test fun t1098InvalidNumericTextAndBoundedOutOfRangeValuesRejectAtomically() {
        val original = ConnectionWorkerSpec("original-device")
        val random = java.util.Random(1098)
        val malformed = listOf("", " ", "1 ", "+1", "-1", "1.0", "1e3", "١", "１", "99999999", "1\u0000")
        for (range in preferenceRanges()) {
            val oversized = List(64) { (range.maximum + 1 + random.nextInt(1000000)).toString() }
            val invalid = malformed + oversized + listOf((range.minimum - 1).toString(), (range.maximum + 1).toString(),
                (4294967296L + range.minimum).toString(), Long.MAX_VALUE.toString())
            for (value in invalid) {
                assertThrows(IllegalArgumentException::class.java) { range.field.update(original, value) }
                val fields = ConnectionPreferences.fields(original) + (range.field.key to value)
                assertThrows(IllegalArgumentException::class.java) { ConnectionPreferences.parse(fields) }
            }
        }
        assertEquals(ConnectionWorkerSpec("original-device"), original)
    }

    @Test fun t1098UnknownMissingAndRetainedMutableInputCannotAlterSnapshot() {
        val fields = ConnectionPreferences.fields(ConnectionWorkerSpec("usb")).toMutableMap()
        val selected = ConnectionPreferences.parse(fields)
        fields["fallback_ms"] = "1"
        assertEquals(100, selected.timerMs)
        assertEquals(1, ConnectionPreferences.parse(fields).timerMs)
        assertThrows(IllegalArgumentException::class.java) { ConnectionPreferences.parse(fields - "quantum") }
        assertThrows(IllegalArgumentException::class.java) { ConnectionPreferences.parse(fields + ("unknown" to "1")) }
        assertThrows(IllegalArgumentException::class.java) { ConnectionPreferences.parse(fields - "quantum" + ("unknown" to "1")) }
        assertThrows(IllegalArgumentException::class.java) { ConnectionPreferences.parse(fields, "") }
    }

    @Test fun t1098MalformedAndOversizedPersistenceRejectBeforeAdmission() {
        val valid = ConnectionPreferences.encode(ConnectionWorkerSpec("usb"))
        val malformed = listOf("", "1", valid.replaceFirst("1\n", "2\n"), valid + "\n", valid + "\nunknown=1",
            valid.replace("channel_bytes=262144", "channel_bytes"),
            valid.replace("queued_messages=64", "channel_bytes=4096"),
            valid.replace("quantum=65536", "unknown=65536"),
            valid.replace("fallback_ms=100", "fallback_ms=1=0"))
        for (value in malformed) assertThrows(IllegalArgumentException::class.java) { ConnectionPreferences.decode(value) }
        val random = java.util.Random(1098)
        repeat(64) {
            val value = "1".repeat(513 + random.nextInt(512))
            assertThrows(IllegalArgumentException::class.java) { ConnectionPreferences.decode(value) }
        }
        assertThrows(IllegalArgumentException::class.java) { ConnectionPreferences.decode("1".repeat(512)) }
    }
}
