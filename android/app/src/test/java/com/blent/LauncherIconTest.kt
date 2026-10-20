package com.blent

import android.graphics.drawable.AdaptiveIconDrawable
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.RuntimeEnvironment
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [27, 34])
class LauncherIconTest {
    @Test fun t1164PackagedLauncherKeepsAdaptiveLayers() {
        val app = RuntimeEnvironment.getApplication()
        val icon = app.getDrawable(R.mipmap.ic_launcher)
        assertTrue("T1164 packaged launcher lost its adaptive resource", icon is AdaptiveIconDrawable)
        val adaptive = icon as AdaptiveIconDrawable
        assertNotNull(adaptive.background)
        assertNotNull(adaptive.foreground)
    }

    @Test @Config(sdk = [34]) fun t1166PackagedLauncherProvidesMonochromeLayer() {
        val app = RuntimeEnvironment.getApplication()
        val icon = app.getDrawable(R.mipmap.ic_launcher) as AdaptiveIconDrawable
        assertNotNull("T1166 themed launcher has no monochrome layer", icon.monochrome)
        assertTrue(icon.monochrome!!.intrinsicWidth > 0)
        assertTrue(icon.monochrome!!.intrinsicHeight > 0)
    }
}
