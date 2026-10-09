package com.blent

import android.media.MediaCodecInfo
import android.media.MediaCodecInfo.CodecProfileLevel as Levels
import android.media.MediaFormat
import kotlinx.coroutines.runBlocking
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config
import org.robolectric.shadows.MediaCodecInfoBuilder

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [27, 34])
class MetadataSnapshotTest {
    private fun capability() = MediaCodecInfoBuilder.CodecCapabilitiesBuilder.newBuilder()
        .setMediaFormat(MediaFormat.createVideoFormat("video/avc", 1920, 1080)).setIsEncoder(false)
        .setColorFormats(intArrayOf(19)).setProfileLevels(arrayOf(Levels().apply {
            profile = Levels.AVCProfileHigh; level = Levels.AVCLevel52
        })).build()
    private fun codec(name: String = "vendor.avc") = MediaCodecInfoBuilder.newBuilder().setName(name)
        .setIsEncoder(false).setIsHardwareAccelerated(true).setIsSoftwareOnly(false)
        .setCapabilities(capability()).build()

    @Test fun t802_reportEnumeratesOnceAndReusesExactMimeCapabilities() = runBlocking {
        val info = codec()
        var enumerations = 0
        val reads = mutableMapOf<String, Int>()
        val report = DecoderCapabilities.report(640, 480, 30, { enumerations++; arrayOf(info) }, { value, mime ->
            reads[mime] = (reads[mime] ?: 0) + 1
            value.getCapabilitiesForType(mime)
        })
        assertEquals("T802 report rebuilt codec inventory per MIME", 1, enumerations)
        assertEquals("T802 compatible decoder metadata queried again for details", 1, reads["video/avc"])
        assertEquals("[\"h264\"]", report.getJSONArray("codecs").toString())
        assertEquals(1, report.getJSONArray("details").length())
    }

    @Test fun t802_duplicateAndUnsupportedProfilesDoNotSpendDetailWorkPastSupportedCap() {
        val caps = capability()
        val levels = VideoProfiles.levels.getValue("h264").size
        val pairs = listOf(Levels.AVCProfileBaseline, Levels.AVCProfileConstrainedBaseline,
            Levels.AVCProfileMain, Levels.AVCProfileHigh).flatMap { profile ->
            (0 until levels).flatMap { bit -> List(4) { Levels().apply { this.profile = profile; level = 1 shl bit } } }
        }
        caps.profileLevels = pairs.toTypedArray()
        var checks = 0
        val report = MediaInventory.describe(codec(), "h264", 640, 480, 30, { caps }, { _, format ->
            checks++
            assertEquals(640, format.getInteger(MediaFormat.KEY_WIDTH))
            assertEquals(480, format.getInteger(MediaFormat.KEY_HEIGHT))
            assertEquals(30, format.getInteger(MediaFormat.KEY_FRAME_RATE))
            format.getInteger(MediaFormat.KEY_PROFILE) in listOf(Levels.AVCProfileMain, Levels.AVCProfileHigh)
        })!!
        assertEquals(32, report.getJSONArray("profiles").length())
        assertEquals("T802 duplicates or entries after output cap still queried support", 2 * levels + 32, checks)
        assertEquals("main", report.getJSONArray("profiles").getJSONObject(0).getString("profile"))
    }
    @Test fun t802_eachReportRescansAndPreservesExactFormatAndFailureResults() = runBlocking {
        val info = codec()
        var reads = 0
        val source = { reads++; arrayOf(info) }
        val capabilities = { value: MediaCodecInfo, mime: String -> value.getCapabilitiesForType(mime) }
        assertEquals(1, DecoderCapabilities.report(640, 480, 30, source, capabilities).getJSONArray("details").length())
        for (format in listOf(intArrayOf(Int.MAX_VALUE, 480, 30), intArrayOf(640, Int.MIN_VALUE, 30), intArrayOf(640, 480, Int.MAX_VALUE))) {
            val report = DecoderCapabilities.report(format[0], format[1], format[2], source, capabilities)
            assertEquals(0, report.getJSONArray("details").length())
        }
        assertEquals(4, reads)
        val absent = DecoderCapabilities.report(640, 480, 30, { error("enumeration failed") }, capabilities)
        assertEquals(0, absent.getJSONArray("codecs").length())
        val failed = DecoderCapabilities.report(640, 480, 30, source, { _, _ -> error("native capability failed") })
        assertEquals(0, failed.getJSONArray("details").length())
        assertEquals(1, DecoderCapabilities.report(640, 480, 30, source, capabilities).getJSONArray("details").length())
    }

    @Test fun t802_detailCapPreservesFirstSupportedUniqueEntriesAfterInvalidNames() = runBlocking {
        val invalid = (0..19).map { codec("bad identity $it") }
        val valid = (0..19).map { codec("vendor.avc.$it") }
        val inventory = (invalid + listOf(valid.first(), valid.first()) + valid).toTypedArray()
        val report = DecoderCapabilities.report(640, 480, 30, { inventory }, { info, mime -> info.getCapabilitiesForType(mime) })
        val details = report.getJSONArray("details")
        assertEquals(16, details.length())
        for (index in 0 until 16) assertEquals("vendor.avc.$index", details.getJSONObject(index).getString("name"))
    }

    @Test fun t802_unknownProfileRangesAndNativeFailuresNeverBecomePositiveClaims() {
        val caps = capability()
        val random = java.util.Random(802)
        caps.profileLevels = (List(256) { Levels().apply { profile = random.nextInt(); level = random.nextInt() } } +
            listOf(Levels().apply { profile = Levels.AVCProfileHigh; level = Int.MIN_VALUE },
                Levels().apply { profile = Levels.AVCProfileHigh; level = Levels.AVCLevel41 })).toTypedArray()
        var checks = 0
        val report = MediaInventory.describe(codec(), "h264", 640, 480, 30, { caps }, { _, _ -> checks++; error("support unknown") })!!
        assertEquals(1, checks)
        assertEquals(0, report.getJSONArray("profiles").length())
        val unknown = MediaInventory.describe(codec(), "h264", 640, 480, 30, { error("details unavailable") })!!
        assertTrue(unknown.isNull("low_latency"))
        assertTrue(unknown.isNull("operating_rate"))
        assertEquals(0, unknown.getJSONArray("profiles").length())
    }

}
