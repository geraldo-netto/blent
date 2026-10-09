package com.blent

import android.media.MediaCodec
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config
import org.robolectric.annotation.Implementation
import org.robolectric.annotation.Implements
import org.robolectric.shadows.ShadowMediaCodec

@Implements(MediaCodec::class)
class RetirementCodecShadow : ShadowMediaCodec() {
    companion object { var failRelease = true; var releases = 0 }
    @Implementation fun stop() { error("stop failed") }
    @Implementation fun release() {
        releases++
        if (failRelease) error("release failed")
    }
}

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [27, 34], shadows = [RetirementCodecShadow::class])
class CodecRetirementFailureTest {
    @Test fun t807_failedReleaseCannotReopenGlobalDecoderAdmission() {
        RetirementCodecShadow.failRelease = true
        RetirementCodecShadow.releases = 0
        val owner = CodecLifetime(MediaCodec.createDecoderByType("video/avc"))
        try {
            owner.retire(); owner.awaitRetirement(2000)
            assertTrue("T807 failed native release reopened decoder admission", CodecLifetime.retirementPending())
            assertFalse(owner.finished)
            assertFalse(CodecLifetime.beginStartup())
            owner.retire(); owner.awaitRetirement(2000)
            assertEquals("T807 uncertain native release retried", 1, RetirementCodecShadow.releases)
        } finally {
            // This sandbox owns no native codec. Remove only its synthetic failed
            // owner so the deliberately unrecoverable fixture cannot poison peers.
            val field = CodecLifetime::class.java.getDeclaredField("retiringOwners").apply { isAccessible = true }
            @Suppress("UNCHECKED_CAST")
            (field.get(null) as MutableSet<CodecLifetime>).remove(owner)
            CodecLifetime.finishStartup()
        }
    }

    @Test fun t807_successfulReleaseAfterStopErrorStillConfirmsRetirement() {
        RetirementCodecShadow.failRelease = false
        val owner = CodecLifetime(MediaCodec.createDecoderByType("video/avc"))
        owner.retire(); owner.awaitRetirement(2000)
        assertTrue(owner.finished)
        assertFalse(CodecLifetime.retirementPending())
    }
}
