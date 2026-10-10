package com.blent.connection

import android.hardware.usb.UsbAccessory
import android.hardware.usb.UsbManager
import android.os.ParcelFileDescriptor

/** Native calls belong on a supervised worker, never the UI/control loop.
 * Every lookup re-enumerates; metadata selects a candidate, never authenticates it. */
internal class AndroidAccessoryInventory(
    manager: UsbManager,
    profile: AccessoryProfile,
    private val capacity: Int = 32,
    private val list: () -> Array<UsbAccessory>? = { manager.accessoryList },
    private val permission: (UsbAccessory) -> Boolean = manager::hasPermission,
    private val opener: (UsbAccessory) -> ParcelFileDescriptor? = manager::openAccessory,
) {
    private val resolver = AccessoryResolver(profile, capacity)
    private fun devices(): List<UsbAccessory> {
        val devices = list() ?: return emptyList()
        require(devices.size <= capacity) { "Accessory snapshot exceeds capacity" }
        return devices.asList()
    }
    fun snapshot(): List<AccessoryDescription> = devices().map(::describe)
    fun resolve(selection: AccessorySelection): UsbAccessory {
        val devices = devices()
        val descriptions = devices.map(::describe)
        val selected = resolver.resolve(selection, descriptions)
        return devices[descriptions.indexOf(selected)]
    }
    fun permitted(selection: AccessorySelection): Boolean = permission(resolve(selection))
    fun open(selection: AccessorySelection): ParcelFileDescriptor {
        val selected = resolve(selection)
        check(permission(selected)) { "Accessory permission is required" }
        return checkNotNull(opener(selected)) { "Accessory opening returned no descriptor" }
    }
    companion object {
        // Android 29+ gates serial behind permission. Keep the same public
        // snapshot before/after approval; duplicate public metadata is ambiguous.
        fun describe(value: UsbAccessory): AccessoryDescription = AccessoryDescription(
            value.manufacturer, value.model, value.description, value.version, value.uri, null,
        )
    }
}
