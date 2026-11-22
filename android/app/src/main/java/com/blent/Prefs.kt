package com.blent

import android.content.Context
import android.content.SharedPreferences

/** Persisted user settings on the tablet side. */
class Prefs(context: Context) {
    private val sp: SharedPreferences =
        context.getSharedPreferences("blent", Context.MODE_PRIVATE)

    companion object {
        private val deliveryGate = Any()
        private val streamSettingsGate = Any()
        /** UI placeholders before host observation; never implicit overrides. */
        const val DEFAULT_BITRATE_KBPS = 20_000
        const val DEFAULT_FPS = 60
        const val DEFAULT_BRIGHTNESS_PERCENT = 50
        const val DEFAULT_DISPLAY_REFRESH_RATE = 60f

        /** Kept in sync with `config::MAX_BITRATE_KBPS` on the host. */
        const val MAX_BITRATE_KBPS = 60_000
        const val MIN_BITRATE_KBPS = 1_000

        /** Values of [orientation]. */
        const val ORIENTATION_AUTO = 0
        const val ORIENTATION_CAMERA_UP = 1
        const val ORIENTATION_CAMERA_DOWN = 2
    }

    internal var streamOverrides: StreamOverrides
        get() = synchronized(streamSettingsGate) {
            StreamOverrides(streamValue("stream_override_bitrate", 1000..60000),
                streamValue("stream_override_fps", 10..90))
        }
        set(value) = synchronized(streamSettingsGate) {
            val editor = sp.edit().remove("stream_override_bitrate").remove("stream_override_fps")
            value.bitrateKbps?.let { editor.putInt("stream_override_bitrate", it) }
            value.fps?.let { editor.putInt("stream_override_fps", it) }
            editor.apply()
        }

    private fun streamValue(key: String, range: IntRange): Int? = try {
        if (sp.contains(key)) sp.getInt(key, 0).takeIf { it in range } else null
    } catch (_: ClassCastException) { null }

    /** Control queue capacity. Smaller values retire congested sessions sooner;
     * larger values tolerate bursts at the cost of retained input/ACK latency. */
    var controlQueueBytes: Int
        get() = sp.getInt("control_queue_bytes", ControlCapacity.DEFAULT).coerceIn(ControlCapacity.MIN, ControlCapacity.MAX)
        set(value) = sp.edit().putInt("control_queue_bytes", value.coerceIn(ControlCapacity.MIN, ControlCapacity.MAX)).apply()

    /** Local window preferences, independent of the host's stream configuration. */
    var brightnessPercent: Int
        get() = sp.getInt("brightness_percent", DEFAULT_BRIGHTNESS_PERCENT).coerceIn(0, 100)
        set(v) = sp.edit().putInt("brightness_percent", v.coerceIn(0, 100)).apply()

    /** Zero follows Android's system preference. */
    var displayRefreshRate: Float
        get() = sp.getFloat("display_refresh_rate", DEFAULT_DISPLAY_REFRESH_RATE)
            .takeIf { it.isFinite() && it >= 0f } ?: DEFAULT_DISPLAY_REFRESH_RATE
        set(v) = sp.edit().putFloat("display_refresh_rate", v).apply()

    /** Opt-in power policy; independent of brightness, refresh and host settings. */
    var batterySaver: Boolean
        get() = sp.getBoolean("battery_saver", false)
        set(v) = sp.edit().putBoolean("battery_saver", v).apply()

    var showStats: Boolean
        get() = sp.getBoolean("show_stats", false)
        set(v) = sp.edit().putBoolean("show_stats", v).apply()

    /**
     * Which way round the tablet is held: [ORIENTATION_AUTO] follows the
     * sensor between the two landscape directions, the other two pin it.
     * Pinning exists because the sensor path does not work everywhere — a
     * Galaxy Tab S9 Ultra with auto-rotate on never left "camera up" — and
     * because people who draw with the camera at the bottom do not want the
     * picture flipping when the tablet is lifted.
     */
    var orientation: Int
        get() = sp.getInt("orientation", ORIENTATION_AUTO)
        set(v) = sp.edit().putInt("orientation", v).apply()

    /**
     * The daemon's session token, delivered as an intent extra when it
     * launches the app over adb. Kept so a relaunch by hand within the same
     * daemon run still authenticates; a new daemon run hands out a new one.
     */
    var hostToken: String?
        get() = sp.getString("host_token", null)
        set(v) = sp.edit().putString("host_token", v).apply()

    /** Monotonic accepted shell-delivery identity, independent of token equality.
     * One process owns this preferences file; a single edit publishes both fields.
     * Exhaustion fails closed rather than reusing a delivery identity. */
    internal val hostTokenDelivery: Long
        get() = sp.getLong("host_token_delivery", 0)

    /** The selection cutoff survives Activity and process replacement. Only a
     * later accepted delivery can restore ADB; absence means untouched startup.
     * Invalid persisted cutoffs fail closed, without reusing delivery identity. */
    internal val adbDeliveryPermitted: Boolean
        get() {
            if (!sp.contains("adb_delivery_cutoff")) return true
            val cutoff = sp.getLong("adb_delivery_cutoff", -1)
            return cutoff >= 0 && hostTokenDelivery > cutoff
        }

    internal fun cutOffAdbDelivery() = synchronized(deliveryGate) {
        sp.edit().putLong("adb_delivery_cutoff", hostTokenDelivery.coerceAtLeast(0)).apply()
    }

    internal fun recordHostTokenDelivery(token: String): Boolean = synchronized(deliveryGate) {
        val previous = hostTokenDelivery
        if (previous < 0 || previous == Long.MAX_VALUE) return false
        sp.edit().putString("host_token", token).putLong("host_token_delivery", previous + 1).apply()
        true
    }

    /** The returned cleanup retains the listener until its owner stops. */
    internal fun observeHostToken(changed: () -> Unit): () -> Unit {
        val listener = SharedPreferences.OnSharedPreferenceChangeListener { _, key ->
            if (key == "host_token" || key == "host_token_delivery") changed()
        }
        sp.registerOnSharedPreferenceChangeListener(listener)
        return { sp.unregisterOnSharedPreferenceChangeListener(listener) }
    }

    /**
     * Whether to ask GitHub for a newer release on an Activity instance
     * starting (at most one completed check per instance unless re-enabled).
     * Stopping cancels an unfinished check; starting can retry it.
     * The host has `check_updates`
     * for the daemon and GUI; this is the
     * tablet's own switch, so the app can be kept fully offline too.
     */
    var checkUpdates: Boolean
        get() = sp.getBoolean("check_updates", true)
        set(v) = sp.edit().putBoolean("check_updates", v).apply()

    /** Shown once, after the first time video actually arrived. */
    var thankedOnce: Boolean
        get() = sp.getBoolean("thanked_once", false)
        set(v) = sp.edit().putBoolean("thanked_once", v).apply()
}
