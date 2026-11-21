package com.blent

import android.media.MediaCodecInfo
import android.media.MediaFormat
import kotlinx.coroutines.runBlocking
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config
import org.robolectric.shadows.MediaCodecInfoBuilder
import org.robolectric.shadows.ShadowMediaCodecList

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [27, 34])
class DecoderEligibilityTest {
    @org.junit.After fun resetInventory() = ShadowMediaCodecList.reset()

    @Test fun t1566RequiredSecureAndTunneledPlaybackNeverBecomeSelectable() = runBlocking {
        for (feature in listOf("secure-playback", "tunneled-playback")) {
            val restricted = codec("vendor.restricted", "video/hevc", arrayOf(feature))
            val normal = codec("vendor.normal", "video/hevc")
            val inventory = arrayOf(restricted, normal)
            val report = DecoderCapabilities.report(640, 480, 30, { inventory }) { info, mime ->
                info.getCapabilitiesForType(mime)
            }
            assertTrue("T1566 missing returned fixture requirement $feature", restricted.getCapabilitiesForType("video/hevc").isFeatureRequired(feature))
            assertEquals("T1566 restricted decoder advertised: $feature", 1, report.getJSONArray("details").length())
            assertEquals("vendor.normal", report.getJSONArray("details").getJSONObject(0).getString("name"))
            assertTrue(decoderReport { arrayOf(restricted) }.contains("hevc=none"))
            ShadowMediaCodecList.reset()
            inventory.forEach(ShadowMediaCodecList::addCodec)
            assertEquals("vendor.normal", DecoderCapabilities.decoderName(
                MediaFormat.createVideoFormat("video/hevc", 640, 480), "video/hevc"))
        }
    }

    @Test fun t1566EveryFamilyRetainsDetailsWhenEarlierFamiliesFillTheirLimit() = runBlocking {
        val inventory = VideoCodec.types.values.flatMap { mime ->
            (0..19).map { codec("vendor.${mime.substringAfter('/')}.$it", mime) }
        }.toTypedArray()
        val report = DecoderCapabilities.report(640, 480, 30, { inventory }) { info, mime ->
            info.getCapabilitiesForType(mime)
        }
        val details = report.getJSONArray("details")
        val families = (0 until details.length()).map { details.getJSONObject(it).getString("codec") }
        assertEquals(16, details.length())
        for (family in listOf("h264", "hevc", "vp9", "av1")) {
            assertEquals("T1566 missing or unbounded family: $family", 4, families.count { it == family })
        }
    }

    @Test fun t1566ShellInventoryUsesTheSameCaseInsensitiveMimeComparison() = runBlocking {
        val info = codec("vendor.hevc", "video/HEVC")
        assertFalse("T1566 shell inventory ignores advertised MIME case", decoderReport { arrayOf(info) }.contains("hevc=none"))
        val report = DecoderCapabilities.report(640, 480, 30, { arrayOf(info) }) { selected, mime ->
            selected.getCapabilitiesForType(mime)
        }
        assertEquals("[\"hevc\"]", report.getJSONArray("codecs").toString())
    }

    @Test fun t1566NamedSelectionRevalidatesOrdinaryPlaybackEligibility() {
        for (feature in listOf("secure-playback", "tunneled-playback")) {
            ShadowMediaCodecList.reset()
            ShadowMediaCodecList.addCodec(codec("vendor.restricted", "video/hevc", arrayOf(feature)))
            val selected = DecoderSelection("vendor.restricted", "hevc", "main", 40, 8, false, null)
            assertThrows("T1566 named selection accepted $feature", IllegalArgumentException::class.java) {
                selected.validate(DecoderFormat("video/hevc", 640, 480, 30))
            }
        }
    }

    // Robolectric 4.17's builder ignores required features below API 29.
    // Populate the real framework flags so API 27 exercises the same contract.
    private fun required(caps: MediaCodecInfo.CodecCapabilities, features: Array<String>) {
        val valid = caps.javaClass.getDeclaredMethod("getValidFeatures").apply { isAccessible = true }
            .invoke(caps) as Array<*>
        val flags = valid.filter { org.robolectric.util.ReflectionHelpers.getField<String>(it, "mName") in features }
            .fold(0) { value, feature -> value or org.robolectric.util.ReflectionHelpers.getField<Int>(feature, "mValue") }
        org.robolectric.util.ReflectionHelpers.setField(caps, "mFlagsRequired", flags)
        features.forEach { assertTrue("T1566 missing fixture requirement $it", caps.isFeatureRequired(it)) }
    }

    private fun profile(mime: String): MediaCodecInfo.CodecProfileLevel {
        val pair = when (mime.lowercase()) {
            "video/avc" -> MediaCodecInfo.CodecProfileLevel.AVCProfileHigh to MediaCodecInfo.CodecProfileLevel.AVCLevel52
            "video/hevc" -> MediaCodecInfo.CodecProfileLevel.HEVCProfileMain to MediaCodecInfo.CodecProfileLevel.HEVCMainTierLevel52
            "video/x-vnd.on2.vp9" -> MediaCodecInfo.CodecProfileLevel.VP9Profile0 to MediaCodecInfo.CodecProfileLevel.VP9Level51
            "video/av01" -> MediaCodecInfo.CodecProfileLevel.AV1ProfileMain8 to MediaCodecInfo.CodecProfileLevel.AV1Level5
            else -> error("T1566 unsupported fixture MIME")
        }
        return MediaCodecInfo.CodecProfileLevel().apply { profile = pair.first; level = pair.second }
    }

    private fun codec(name: String, mime: String, required: Array<String> = emptyArray()): MediaCodecInfo {
        val caps = MediaCodecInfoBuilder.CodecCapabilitiesBuilder.newBuilder()
            .setMediaFormat(MediaFormat.createVideoFormat(mime, 1920, 1080).apply {
                required.forEach { setFeatureEnabled(it, true) }
            })
            .setIsEncoder(false).setColorFormats(intArrayOf(19))
            .setProfileLevels(arrayOf(profile(mime))).setRequiredFeatures(required).build()
        required(caps, required)
        return MediaCodecInfoBuilder.newBuilder().setName(name).setIsEncoder(false)
            .setIsSoftwareOnly(false).setIsHardwareAccelerated(true).setCapabilities(caps).build()
    }
}
