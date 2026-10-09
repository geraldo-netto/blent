package com.blent

import android.media.MediaCodec
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [27, 34])
class DecoderDeadlineTest {
    @Test fun t806_startupGatePublishesIndependentWaitStatus() {
        assertTrue(CodecLifetime.beginStartup())
        try {
            assertNotNull("T806 native startup can block without observable wait state", CodecLifetime.startupStatus.value)
            assertFalse(CodecLifetime.beginStartup())
        } finally { CodecLifetime.finishStartup() }
        assertNull(CodecLifetime.startupStatus.value)
    }

    @Test fun t806_retirementGatePublishesWaitUntilBorrowerReturns() {
        val borrowed = CountDownLatch(1)
        val release = CountDownLatch(1)
        val owner = CodecLifetime(MediaCodec.createDecoderByType("video/avc"))
        val thread = Thread { owner.use { borrowed.countDown(); release.await() } }.apply { start() }
        try {
            assertTrue(borrowed.await(2, TimeUnit.SECONDS))
            owner.retire()
            assertNotNull("T806 native retirement can block without observable wait state", CodecLifetime.retirementStatus.value)
            assertFalse(CodecLifetime.beginStartup())
            assertFalse(owner.finished)
        } finally {
            release.countDown(); thread.join(2000); owner.awaitRetirement(2000)
        }
        assertTrue(owner.finished)
        assertNull(CodecLifetime.retirementStatus.value)
    }
}
