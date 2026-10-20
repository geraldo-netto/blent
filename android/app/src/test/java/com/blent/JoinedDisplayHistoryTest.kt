package com.blent

import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [27, 34], shadows = [JoinedDisplayCodecShadow::class])
class JoinedDisplayHistoryTest {
    @get:org.junit.Rule val inventory = DecoderInventoryRule()

    @Test fun t1172OriginalNativeCloseReturnsTheBoundedShadowHistoryCapacity() {
        JoinedDisplayActivityTest().t1011CurrentRouteResumesOnlyAfterOriginalNativeFenceWithFreshCodecHeadersAndKeyframe()
        assertTrue("T1172 original codec release did not complete", JoinedDisplayCodecShadow.instances.all { it.released })
        assertTrue("T1172 closed fixture still consumes bounded shadow capacity", JoinedDisplayCodecShadow.instances.isEmpty())
    }
}
