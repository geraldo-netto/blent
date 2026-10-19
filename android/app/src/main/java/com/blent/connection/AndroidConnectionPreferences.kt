package com.blent.connection

import android.content.Context

/** The application owns this store for its process lifetime. Context and native
 * preference access stay in this adapter; the worker process receives only spec. */
internal object AndroidConnectionPreferences {
    private var store: ConnectionPreferenceStore? = null
    @Synchronized fun get(context: Context): ConnectionPreferenceStore = store ?: ConnectionPreferenceStore(
        AndroidConnectionPreferenceStorage(context),
    ).also { store = it }
}

internal class AndroidConnectionPreferenceStorage(context: Context) : ConnectionPreferenceStorage {
    private val context = context.applicationContext
    private fun preferences() = context.getSharedPreferences("usb_connection", Context.MODE_PRIVATE)
    override fun read(): String? = preferences().getString("settings", null)
    override fun write(encoded: String): Boolean {
        require(encoded.length in 1..ConnectionPreferences.MAX_ENCODING)
        return preferences().edit().putString("settings", encoded).commit()
    }
}
