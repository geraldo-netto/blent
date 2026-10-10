package com.blent.connection

import android.os.Looper
import com.blent.*
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.Robolectric
import org.robolectric.RobolectricTestRunner
import org.robolectric.Shadows.shadowOf
import org.robolectric.annotation.Config
import org.robolectric.annotation.LooperMode
import org.robolectric.util.ReflectionHelpers

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [27, 34], instrumentedPackages = ["androidx.compose.ui.platform"])
@LooperMode(LooperMode.Mode.PAUSED)
class ConnectionDisplayActivityTest {
    private fun select(fixture: ConnectionUiFixture): LocalDisplayRoute {
        fixture.connect(); fixture.prompt()
        val worker = fixture.workers.single()
        worker.report(ConnectionWorkerEvent.State(ConnectionWorkerPhase.AUTHENTICATED))
        return displayRoute().also { worker.report(ConnectionWorkerEvent.Display(it)) }
    }
    @Test fun t907StoppedActivityRechecksLatestLeaseBeforeRestartingEitherChannel() {
        ConnectionUiFixture().use { fixture ->
            WorkerTestBridge.install(); val rig = SessionDisplayRig(); val route = select(fixture)
            val controller = Robolectric.buildActivity(MainActivity::class.java).create()
            ReflectionHelpers.setField(controller.get(), "session", rig.session)
            try {
                controller.start().resume(); shadowOf(Looper.getMainLooper()).idle()
                assertEquals(route.controlPort, rig.sockets.single().first.request().url.port)
                controller.pause().stop()
                fixture.workers.single().report(ConnectionWorkerEvent.DisplayClosed(route.lease))
                shadowOf(Looper.getMainLooper()).idle()
                assertEquals(1, rig.sockets.size); assertTrue(rig.sockets.single().first.cancelled)
                controller.start().resume(); shadowOf(Looper.getMainLooper()).idle()
                assertNull(rig.control.routes); assertNull(rig.receiver.routes); assertEquals(1, rig.sockets.size)
                assertEquals(0, fixture.workers.single().cancelled)
                val next = displayRoute(2)
                fixture.workers.single().report(ConnectionWorkerEvent.Display(next)); shadowOf(Looper.getMainLooper()).idle()
                assertEquals(2, rig.sockets.size); assertEquals(next.controlPort, rig.control.routes?.controlPort)
                controller.pause().stop()
            } finally { controller.destroy(); CameraOwner.reset(); AudioOwner.reset() }
        }
    }
    @Test fun t907ActivityRecreationRetainsConnectionButCreatesOnlyCurrentLocalRouteBindings() {
        ConnectionUiFixture().use { fixture ->
            WorkerTestBridge.install(); val firstRig = SessionDisplayRig(); val route = select(fixture)
            val first = Robolectric.buildActivity(MainActivity::class.java).create()
            ReflectionHelpers.setField(first.get(), "session", firstRig.session)
            first.start().resume(); shadowOf(Looper.getMainLooper()).idle()
            first.pause().stop().destroy()
            assertTrue(firstRig.sockets.single().first.cancelled)
            val secondRig = SessionDisplayRig()
            val second = Robolectric.buildActivity(MainActivity::class.java).create()
            ReflectionHelpers.setField(second.get(), "session", secondRig.session)
            try {
                second.start().resume(); shadowOf(Looper.getMainLooper()).idle()
                assertSame(fixture.binding, second.get().connection); assertEquals(1, fixture.workers.size)
                assertEquals(route.controlPort, secondRig.sockets.single().first.request().url.port)
                fixture.binding.disconnect(); shadowOf(Looper.getMainLooper()).idle()
                assertNull(secondRig.control.routes); assertNull(secondRig.receiver.routes)
                assertEquals(1, firstRig.sockets.size); assertEquals(1, secondRig.sockets.size)
                second.pause().stop()
            } finally { second.destroy(); CameraOwner.reset(); AudioOwner.reset() }
        }
    }
    @Test fun t907ActiveConnectionWithoutDisplayNeverStartsSavedAdbFromNewActivity() {
        ConnectionUiFixture().use { fixture ->
            WorkerTestBridge.install(); val rig = SessionDisplayRig(); fixture.connect()
            val controller = Robolectric.buildActivity(MainActivity::class.java).create()
            ReflectionHelpers.setField(controller.get(), "session", rig.session)
            try {
                controller.start().resume(); shadowOf(Looper.getMainLooper()).idle()
                assertTrue(rig.sockets.isEmpty()); assertNull(rig.control.routes); assertNull(rig.receiver.routes)
                controller.pause().stop()
            } finally { controller.destroy(); CameraOwner.reset(); AudioOwner.reset() }
        }
    }
}
