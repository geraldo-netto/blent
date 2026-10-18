package com.blent

import android.os.Looper
import android.view.WindowManager
import java.util.concurrent.ConcurrentLinkedQueue
import kotlinx.coroutines.*
import kotlinx.coroutines.flow.MutableStateFlow
import okhttp3.*
import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.Robolectric
import org.robolectric.RobolectricTestRunner
import org.robolectric.RuntimeEnvironment
import org.robolectric.Shadows.shadowOf
import org.robolectric.annotation.Config
import org.robolectric.util.ReflectionHelpers

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [27, 34])
class DisplayWakeIntegrationTest {
    internal class Rig(receiver: DisplaySession<android.view.SurfaceView>? = null) {
        val sockets = ArrayList<Pair<DisplayRouteSocket, WebSocketListener>>()
        val control = TouchCapture(WebSocket.Factory { request, listener ->
            DisplayRouteSocket(request).also { sockets.add(it to listener) }
        }, { _, _, _ -> JSONObject() })
        val actions = ConcurrentLinkedQueue<() -> Unit>()
        val prefs = Prefs(RuntimeEnvironment.getApplication()).apply { hostToken = "ab".repeat(32); checkUpdates = false }
        val session = SessionCoordinator(prefs, { actions.add(it) }, receiver, control)
        fun start() {
            session.start()
            opened()
        }
        fun opened() {
            val (socket, listener) = sockets.last()
            listener.onOpen(socket, Response.Builder().request(socket.request()).protocol(Protocol.HTTP_1_1).code(101).message("T1061 original").build())
        }
        fun signal(active: Boolean, revision: Long = 1, stopped: Long = 0, greeting: Boolean = true) {
            val (socket, listener) = sockets.last()
            listener.onMessage(socket, JSONObject().put("status", if (greeting) "connected" else "display_activity").put("codec", "h264")
                .put("video_width", 64).put("video_height", 32).put("fps", 30).put("pen_only", false)
                .put("display_activity", JSONObject().put("version", 1).put("revision", revision).put("stopped", stopped).put("active", active)).toString())
        }
        fun drain() { while (true) (actions.poll() ?: break).invoke(); shadowOf(Looper.getMainLooper()).idle() }
        suspend fun until(extra: () -> Unit = {}, predicate: () -> Boolean) {
            withTimeout(4000) {
                while (!predicate()) {
                    androidx.compose.runtime.snapshots.Snapshot.sendApplyNotifications()
                    drain(); extra(); delay(1)
                }
            }
            drain(); extra()
        }
        suspend fun close() {
            session.stop(); drain()
            val scope = ReflectionHelpers.getField<CoroutineScope>(control.control, "scope")
            checkNotNull(scope.coroutineContext[Job]).cancelAndJoin()
        }
    }
    private fun windowObserver(binding: WindowPowerBinding): Job {
        val original = ReflectionHelpers.getField<Any>(binding, "current")
        val scope = ReflectionHelpers.getField<CoroutineScope>(original, "scope")
        return checkNotNull(scope.coroutineContext[Job])
    }
    @Test fun t1061PausedSourceCannotRequestWakeAheadOfItsQueuedUiDelivery() = runBlocking {
        val rig = Rig()
        try {
            rig.start(); assertTrue(rig.session.powerNow().keepScreenOn)
            rig.signal(false)
            assertTrue(rig.control.connectionState.value.authenticated)
            assertFalse("T1061 paused original source requested screen wake", rig.session.powerNow().keepScreenOn)
            assertFalse(rig.session.powerNow().needsCpu)
            assertFalse(rig.session.powerNow().needsWifi)
        } finally { rig.close() }
    }
    @Test fun t1061BackgroundCannotRetainWaitingWakeAfterSourceClosure() = runBlocking {
        val rig = Rig()
        try {
            rig.start(); rig.signal(true)
            rig.session.stop()
            assertFalse("T1061 stopped foreground retained wake", rig.session.powerNow().keepScreenOn)
            assertEquals(DisplayWakeState.BACKGROUND, rig.session.powerNow().display)
            rig.drain()
            assertFalse(rig.session.powerNow().keepScreenOn)
        } finally { rig.close() }
    }
    @Test fun t1061JoinedPowerAndWindowFollowSourceAcrossActualHeldNativeRetirement() = runBlocking {
        val rig = Rig()
        val power = PowerRuntimeFixture(hold = true)
        val binding = StreamingPowerBinding(power.app, power.runtime)
        val writes = ArrayList<Boolean>()
        val window = WindowPowerBinding(writes::add)
        binding.start(rig.session.powerDemandNow(), rig.session.powerDemandUpdates())
        window.start(rig.session.powerDemandNow(), rig.session.powerDemandUpdates())
        shadowOf(Looper.getMainLooper()).idle()
        val nativeObserver = checkNotNull(ReflectionHelpers.getField<CoroutineScope>(binding, "scope").coroutineContext[Job])
        val originalWindow = windowObserver(window)
        val hints = ReflectionHelpers.getField<MutableStateFlow<com.blent.activity.DisplayActivityUpdate>>(rig.control.control, "activityHints")
        val owner = checkNotNull(PowerServiceNotice.read(power.app.starts.single())).notice.original.owner
        try {
            rig.until { hints.subscriptionCount.value == 2 }
            assertFalse(writes.last())
            rig.start(); rig.signal(false)
            rig.until { rig.session.powerNow().display == DisplayWakeState.PAUSED }
            power.attachFirst(); assertTrue(power.native.isEmpty())
            rig.signal(true, 2, 0, false)
            rig.until(power::drain) { power.native.size == 1 && writes.last() }
            rig.signal(false, 3, 1, false)
            rig.until(power::drain) { !writes.last() && power.canceled > 0 }
            assertEquals(0, power.settled)
            rig.signal(true, 4, 1, false)
            rig.until(power::drain) { writes.last() }
            assertEquals("T1061 source resume bypassed actual original retirement", 1, power.native.size)
            power.release.complete(Unit)
            rig.until(power::drain) { power.native.size == 2 }
            assertEquals(1, power.settled)
        } finally {
            power.release.complete(Unit); window.stop(); binding.stop(); rig.close(); power.finish(owner)
        }
        rig.until { nativeObserver.isCompleted && originalWindow.isCompleted && hints.subscriptionCount.value == 0 }
        assertFalse(writes.last())
    }
    @Test fun t1061CoalescedLossAndFreshLegacyFallbackDoNotDependOnOptionalUiDelivery() = runBlocking {
        val rig = Rig()
        val writes = ArrayList<Boolean>()
        val window = WindowPowerBinding(writes::add)
        window.start(rig.session.powerDemandNow(), rig.session.powerDemandUpdates())
        try {
            rig.start(); rig.signal(true)
            rig.until { writes.last() }
            rig.control.onDisplayActivityKnown = { error("T1061 optional UI delivery failed") }
            rig.control.disconnect(); rig.start()
            rig.until { !writes.last() }
            assertTrue(rig.control.displayActivityUpdate.supportKnown)
            val (socket, listener) = rig.sockets.last()
            listener.onMessage(socket, """{"status":"connected","codec":"h264","fps":30,"video_width":64,"video_height":32,"pen_only":false}""")
            rig.until { writes.last() }
            assertEquals(DisplayWakeState.UNSUPPORTED, rig.session.powerNow().display)
        } finally { window.stop(); rig.close() }
        assertFalse(writes.last())
    }
    @Test fun t1061ActualActivityWindowReleasesPauseAndBackgroundWhileRetainingSettings() = runBlocking {
        val rig = Rig()
        val power = PowerRuntimeFixture()
        val controller = Robolectric.buildActivity(MainActivity::class.java).create()
        val activity = controller.get()
        activity.session.stop()
        ReflectionHelpers.getField<StreamingPowerBinding>(activity, "powerBinding").stop()
        ReflectionHelpers.setField(activity, "session", rig.session)
        ReflectionHelpers.setField(activity, "powerBinding", StreamingPowerBinding(power.app, power.runtime))
        fun awake() = activity.window.attributes.flags and WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON != 0
        var owner: DisplayPowerOwner? = null
        try {
            controller.start(); rig.opened()
            owner = checkNotNull(PowerServiceNotice.read(power.app.starts.single())).notice.original.owner
            rig.until { awake() }
            power.attachFirst()
            rig.signal(false)
            rig.until(power::drain) { !awake() }
            val brightness = rig.prefs.brightnessPercent
            rig.session.handle(SettingsEvent.BatterySaver(true)); rig.session.handle(SettingsEvent.RefreshRate(60f))
            rig.drain(); power.drain(); assertFalse(awake())
            assertEquals(brightness, rig.prefs.brightnessPercent)
            rig.signal(true, 2, 0, false)
            rig.until(power::drain) { awake() }
            assertTrue(rig.prefs.batterySaver)
            controller.stop(); rig.drain(); power.drain()
            assertFalse("T1061 actual stopped Window retained its flag", awake())
        } finally {
            controller.destroy(); rig.close()
            val original = owner
            if (original != null) power.finish(original) else power.scope.cancel()
        }
    }
    @Test fun t1061CurrentMetadataAndExistingPenModeCannotOverridePausedSource() = runBlocking {
        val rig = Rig()
        try {
            rig.start(); rig.signal(true)
            rig.until { rig.session.powerNow().display == DisplayWakeState.ACTIVE }
            rig.control.onStreamFormat!!.invoke(null)
            rig.until { rig.session.powerNow().display == DisplayWakeState.UNAVAILABLE }
            rig.control.onCodecKnown!!.invoke("h264"); rig.drain()
            assertFalse(rig.session.powerNow().keepScreenOn)
            rig.control.onModeKnown!!.invoke(true)
            rig.until { rig.session.powerNow().display == DisplayWakeState.ACTIVE }
            assertTrue(rig.session.powerNow().active)
            rig.signal(false, 2, 1, false)
            rig.session.handle(SettingsEvent.BatterySaver(true))
            rig.until { !rig.session.powerNow().keepScreenOn }
            assertFalse(rig.session.powerNow().needsCpu); assertFalse(rig.session.powerNow().needsWifi)
        } finally { rig.close() }
    }
    @Test fun t1061CreatedActivityDoesNotOwnScreenWakeBeforeForegroundStart() {
        Prefs(RuntimeEnvironment.getApplication()).checkUpdates = false
        val controller = Robolectric.buildActivity(MainActivity::class.java).create()
        try {
            val flags = controller.get().window.attributes.flags
            assertEquals("T1061 onCreate unconditionally claimed screen wake", 0, flags and WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
        } finally { controller.destroy() }
    }
}
