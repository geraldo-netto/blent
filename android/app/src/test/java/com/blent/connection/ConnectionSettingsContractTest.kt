package com.blent.connection

import org.junit.Assert.*
import org.junit.Test

internal data class UsbSettingRange(
    val field: String, val hostOption: String, val default: Long, val minimum: Long, val maximum: Long,
)
private object UsbSettingsFixture

private fun usbSettingRange(line: String): UsbSettingRange {
    val fields = line.split('\t')
    check(fields.size == 5)
    return UsbSettingRange(fields[0], fields[1], fields[2].toLong(), fields[3].toLong(), fields[4].toLong())
}
internal fun usbSettingsContract(): List<UsbSettingRange> {
    val source = checkNotNull(UsbSettingsFixture.javaClass.getResourceAsStream("/connection-settings-v1.tsv"))
    return source.bufferedReader().use { reader ->
        val lines = reader.readLines()
        check(lines.size == 10)
        check(lines.first() == "android_field\thost_option\tdefault\tminimum\tmaximum")
        lines.drop(1).map(::usbSettingRange)
    }
}

class ConnectionSettingsContractTest {
    @Test fun t898SharedHostAndroidDefaultsBoundariesAndEncodingAgree() {
        val original = ConnectionWorkerSpec("current-device")
        val defaults = ConnectionPreferences.fields(original)
        for (range in usbSettingsContract()) {
            assertEquals(range.default.toString(), defaults[range.field])
            for (value in listOf(range.minimum, range.maximum)) {
                val fields = defaults + (range.field to value.toString())
                val selected = ConnectionPreferences.parse(fields, original.device)
                assertEquals(fields, ConnectionPreferences.fields(selected))
                assertEquals(selected, ConnectionPreferences.decode(ConnectionPreferences.encode(selected), original.device))
                assertEquals(defaults, ConnectionPreferences.fields(original))
            }
        }
    }

    @Test fun t898SharedOutOfBoundsValuesCannotPublishPartialOrReplaceCurrentSnapshot() {
        val current = ConnectionWorkerSpec("current-device")
        val snapshot = ConnectionPreferences.fields(current)
        val encoded = ConnectionPreferences.encode(current)
        for (range in usbSettingsContract()) {
            for (value in listOf(range.minimum - 1, range.maximum + 1)) {
                assertThrows(IllegalArgumentException::class.java) {
                    ConnectionPreferences.parse(snapshot + (range.field to value.toString()), current.device)
                }
                assertEquals(snapshot, ConnectionPreferences.fields(current))
                assertEquals(encoded, ConnectionPreferences.encode(current))
            }
        }
    }
}
