package com.blent.connection

import com.blent.*
import java.time.Duration
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.Robolectric
import org.robolectric.RuntimeEnvironment
import org.robolectric.Shadows.shadowOf
import org.robolectric.annotation.Config

@RunWith(com.blent.ComposeFixtureRunner::class)
@Config(sdk = [27, 34], instrumentedPackages = ["androidx.compose.ui.platform"])
class RememberedConnectionActivityTest {
    @Test fun t1393AuthenticatedRememberedSessionRetriesWithoutAnotherAttachIntent() {
        ConnectionUiFixture().use { fixture ->
            WorkerTestBridge.install()
            Prefs(RuntimeEnvironment.getApplication()).checkUpdates = false
            val activity = Robolectric.buildActivity(MainActivity::class.java).create()
            try {
                fixture.connect()
                val original = fixture.workers.single()
                original.report(ConnectionWorkerEvent.Peer(fixture.ticket, fixture.tablet, true))
                original.report(ConnectionWorkerEvent.State(ConnectionWorkerPhase.AUTHENTICATED))
                original.released.complete(Unit)
                shadowOf(android.os.Looper.getMainLooper()).idleFor(Duration.ofSeconds(2))
                assertEquals("T1393: remembered session requires another attach or manual Connect", 2, fixture.workers.size)
                assertTrue(fixture.workers.last().commands.isEmpty())
            } finally { activity.destroy(); CameraOwner.reset(); AudioOwner.reset() }
        }
    }
}
