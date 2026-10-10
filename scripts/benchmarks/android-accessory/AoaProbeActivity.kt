package com.blent

import android.app.Activity
import android.app.PendingIntent
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.hardware.usb.UsbAccessory
import android.hardware.usb.UsbManager
import android.os.Build
import android.os.Bundle
import android.view.WindowManager
import android.widget.Button
import android.widget.LinearLayout
import android.widget.TextView

/** Profile-only UI in its own process. It never accesses Blent tokens or media. */
class AoaProbeActivity : Activity() {
    private lateinit var status: TextView
    private lateinit var manager: UsbManager
    private var session: AoaProbeRun? = null
    private val permissionAction get() = "$packageName.AOA_PROBE_PERMISSION"
    private val receiver = object : BroadcastReceiver() {
        override fun onReceive(context: Context, intent: Intent) {
            if (intent.action == UsbManager.ACTION_USB_ACCESSORY_DETACHED) stopProbe()
            if (intent.action == permissionAction) startProbe(false)
        }
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        manager = getSystemService(UsbManager::class.java)
        window.addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
        showControls()
        val filter = IntentFilter(permissionAction).apply { addAction(UsbManager.ACTION_USB_ACCESSORY_DETACHED) }
        if (Build.VERSION.SDK_INT >= 33) registerReceiver(receiver, filter, RECEIVER_NOT_EXPORTED)
        else registerReceiver(receiver, filter)
        startProbe(false)
    }

    private fun showControls() {
        val layout = LinearLayout(this).apply { orientation = LinearLayout.VERTICAL; setPadding(32, 32, 32, 32) }
        status = TextView(this).apply {
            text = "USB accessory test only. No display or input authority.\nStart the host probe, then approve accessory access.\nUnplug/replug after testing to restore normal USB mode."
            textSize = 20f
        }
        layout.addView(status)
        layout.addView(Button(this).apply { text = "Start USB test"; setOnClickListener { startProbe(true) } })
        layout.addView(Button(this).apply { text = "Stop USB test"; setOnClickListener { stopProbe() } })
        setContentView(layout)
    }

    private fun accessory(): UsbAccessory? = manager.accessoryList?.singleOrNull {
        it.manufacturer == "Blent" && it.model == "Blent USB Probe" && it.version == "1"
    }

    private fun startProbe(requestPermission: Boolean) {
        if (session != null) return
        val accessory = accessory() ?: return
        if (!manager.hasPermission(accessory)) {
            status.text = "Accessory permission required. Tap Start USB test, then approve Android's prompt."
            if (requestPermission) askPermission(accessory)
            return
        }
        status.text = "USB test running; maximum 60 seconds. Stop affects only this probe process."
        session = AoaProbeRun(this, manager, accessory) { message ->
            runOnUiThread { status.text = message }
        }.also {
            if (!it.start()) status.text = "One test per process. Tap Stop, then reopen Blent USB Probe."
        }
    }

    private fun askPermission(accessory: UsbAccessory) {
        val intent = Intent(permissionAction).setPackage(packageName)
        val pending = PendingIntent.getBroadcast(this, 846, intent, PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT)
        manager.requestPermission(accessory, pending)
    }

    private fun stopProbe() {
        session?.stop()
        status.text = "Probe stopped. Unplug/replug to restore normal USB mode."
    }

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        startProbe(false)
    }

    override fun onStop() {
        session?.stop()
        super.onStop()
    }

    override fun onDestroy() {
        unregisterReceiver(receiver)
        session?.stop()
        super.onDestroy()
    }
}
