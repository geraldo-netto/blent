package com.blent

import android.content.Context

internal object AudioPreferencesStore {
    fun save(value: AudioPreferences, context: Context, direction: Int = 1) {
        require(value.valid(direction))
        context.getSharedPreferences(namespace(direction), Context.MODE_PRIVATE).edit()
            .putInt("processing", value.processing).putBoolean("background", value.background)
            .putInt("gain", value.gain).putBoolean("built_in", value.builtIn).apply()
    }
    private fun namespace(direction: Int): String {
        require(direction in 1..2)
        return if (direction == 1) "audio" else "audio_speakers"
    }
    fun load(context: Context, direction: Int = 1): AudioPreferences {
        val prefs = context.getSharedPreferences(namespace(direction), Context.MODE_PRIVATE)
        return AudioPreferences(prefs.getInt("processing", 0).coerceIn(0, 2), prefs.getBoolean("background", false),
            prefs.getInt("gain", 100).coerceIn(0, if (direction == 1) 200 else 100), prefs.getBoolean("built_in", true))
    }
}

internal fun AudioPreferences.save(context: Context, direction: Int = 1) = AudioPreferencesStore.save(this, context, direction)
