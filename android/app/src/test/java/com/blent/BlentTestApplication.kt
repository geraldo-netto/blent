// Copyright (c) 2026 Geraldo Netto
package com.blent

/** Preserve dispatcher/queue consistency at each Robolectric sandbox reset. */
class BlentTestApplication : BlentApplication() {
    override fun onTerminate() {
        org.robolectric.Shadows.shadowOf(android.os.Looper.getMainLooper()).runToEndOfTasks()
        super.onTerminate()
    }
}
