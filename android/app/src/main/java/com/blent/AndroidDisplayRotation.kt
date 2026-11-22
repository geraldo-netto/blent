package com.blent

import android.content.Context
import android.hardware.display.DisplayManager
import android.os.Handler
import android.os.Looper
import android.view.Display

/** Application-only adapter; never retains an Activity or acquires camera access. */
internal class AndroidDisplayRotation(context: Context) : DisplayRotationSource {
    private val manager = context.applicationContext.getSystemService(DisplayManager::class.java)
    private var owner: Observation? = null

    override fun current(): Int = manager.getDisplay(Display.DEFAULT_DISPLAY)?.rotation ?: 0

    override fun start(changed: () -> Unit) {
        check(owner == null) { "Display rotation observation already owned" }
        val original = Observation(changed)
        owner = original // Keep exact registration identity even if the framework throws.
        manager.registerDisplayListener(original, Handler(Looper.getMainLooper()))
    }

    override fun stop() {
        val original = owner ?: return
        original.active = false
        manager.unregisterDisplayListener(original)
        owner = null
    }

    private inner class Observation(private val changed: () -> Unit) : DisplayManager.DisplayListener {
        var active = true
        override fun onDisplayAdded(displayId: Int) = onDisplayChanged(displayId)
        override fun onDisplayRemoved(displayId: Int) {}
        override fun onDisplayChanged(displayId: Int) {
            if (active && owner === this && displayId == Display.DEFAULT_DISPLAY) changed()
        }
    }
}
