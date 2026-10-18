package com.blent

import android.app.Application
import java.lang.ref.WeakReference
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RuntimeEnvironment
import org.robolectric.RobolectricTestRunner
import org.robolectric.Shadows.shadowOf
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [27, 34])
class PowerApplicationOwnerTest {
    private class Pending(val root: WeakReference<StreamingPowerRuntime>, val notice: DisplayPowerNotice)
    private fun abandonedProducer(app: Application): Pending {
        val runtime = StreamingPowerRuntime.forApp(app)
        val owner = runtime.open(StreamingPower())
        runtime.close(owner)
        // The native model has not delivered or fenced this original admission.
        // Remove its retained Intent so only the Application can sponsor cleanup.
        val intent = checkNotNull(shadowOf(app).nextStartedService)
        val notice = checkNotNull(PowerServiceNotice.read(intent)).notice
        assertNull(shadowOf(app).nextStartedService)
        return Pending(WeakReference(runtime), notice)
    }
    @Test fun t1060ApplicationRetainsOriginalUnknownHandoffAfterForegroundProducerLoss() {
        val app = RuntimeEnvironment.getApplication()
        val pending = abandonedProducer(app)
        try {
            repeat(16) { System.gc(); System.runFinalization() }
            val original = pending.root.get()
            assertNotNull("T1060 application forgot its outstanding original handoff", original)
            assertSame("T1060 a second runtime bypassed retained admission", original, StreamingPowerRuntime.forApp(app))
        } finally { pending.root.get()?.canceled(pending.notice) }
    }
}
