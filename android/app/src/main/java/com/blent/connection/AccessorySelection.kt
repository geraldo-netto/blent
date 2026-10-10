package com.blent.connection

import java.nio.ByteBuffer
import java.security.MessageDigest

/** USB metadata is selection evidence only, never authenticated peer identity.
 * Null and empty are distinct. Cached digest avoids rehashing the same snapshot. */
internal data class AccessoryDescription(
    val manufacturer: String?, val model: String?, val description: String?,
    val version: String?, val uri: String?, val serial: String?,
) {
    val digest: String = AccessoryMetadata.digest(listOf(manufacturer, model, description, version, uri, serial))
}

internal object AccessoryMetadata {
    const val MAX_FIELD_BYTES = 512
    private fun bytes(value: String?): ByteArray? {
        if (value == null) return null
        require(value.length <= MAX_FIELD_BYTES)
        require(Charsets.UTF_8.newEncoder().canEncode(value)) { "Malformed accessory Unicode" }
        return value.toByteArray(Charsets.UTF_8).also { require(it.size <= MAX_FIELD_BYTES) }
    }
    fun digest(fields: List<String?>): String {
        require(fields.size == 6)
        val hash = MessageDigest.getInstance("SHA-256")
        val length = ByteBuffer.allocate(4)
        for (field in fields) {
            val encoded = bytes(field)
            length.clear(); length.putInt(encoded?.size ?: -1)
            hash.update(length.array())
            if (encoded != null) hash.update(encoded)
        }
        return hash.digest().joinToString("") { "%02x".format(it.toInt() and 255) }
    }
}

internal data class AccessoryProfile(val manufacturer: String, val model: String, val version: String) {
    init {
        require(manufacturer.isNotEmpty() && model.isNotEmpty() && version.isNotEmpty())
        AccessoryMetadata.digest(listOf(manufacturer, model, null, version, null, null))
    }
    fun matches(value: AccessoryDescription): Boolean =
        manufacturer == value.manufacturer && model == value.model && version == value.version
}

/** Epoch prevents local stale callbacks. It cannot distinguish physical devices
 * that expose identical metadata; detach handling and fresh permission checks
 * remain mandatory. This token never grants TLS or media authority. */
internal data class AccessorySelection(val epoch: Long, val digest: String) {
    init {
        require(epoch > 0)
        require(digest.length == 64 && digest.all { it in '0'..'9' || it in 'a'..'f' })
    }
    fun encode(): String = "$epoch:$digest"
    companion object {
        fun parse(token: String): AccessorySelection {
            require(token.length in 66..84)
            val separator = token.indexOf(':')
            require(separator in 1..19)
            val text = token.substring(0, separator)
            val epoch = requireNotNull(text.toLongOrNull())
            require(text == epoch.toString()) { "Noncanonical accessory epoch" }
            return AccessorySelection(epoch, token.substring(separator + 1))
        }
    }
}

/** Native adapters provide a fresh bounded snapshot; policy performs no I/O.
 * A worker can resolve a token but cannot establish main-owner currentness. */
internal class AccessoryResolver(private val profile: AccessoryProfile, private val capacity: Int = 32) {
    init { require(capacity in 1..1024) }
    private fun bounded(snapshot: List<AccessoryDescription>) { require(snapshot.size <= capacity) }
    fun choose(snapshot: List<AccessoryDescription>): AccessoryDescription {
        bounded(snapshot)
        var selected: AccessoryDescription? = null
        for (value in snapshot) {
            if (!profile.matches(value)) continue
            check(selected == null) { "Ambiguous supported accessories" }
            selected = value
        }
        return checkNotNull(selected) { "No supported accessory" }
    }
    fun resolve(selection: AccessorySelection, snapshot: List<AccessoryDescription>): AccessoryDescription {
        bounded(snapshot)
        var selected: AccessoryDescription? = null
        for (value in snapshot) {
            if (value.digest != selection.digest) continue
            check(selected == null) { "Ambiguous accessory snapshot" }
            check(profile.matches(value)) { "Unsupported accessory profile" }
            selected = value
        }
        return checkNotNull(selected) { "Selected accessory disappeared" }
    }
}

/** One serialized owner; replacement and detach invalidate the previous token.
 * current checks only local selection lifetime. Native permission must be freshly
 * checked by the adapter before confirm and immediately before/after opening. */
internal class AccessorySelectionOwner(
    profile: AccessoryProfile, capacity: Int = 32, initialEpoch: Long = 0,
) {
    private val resolver = AccessoryResolver(profile, capacity)
    private var epoch = initialEpoch
    private var selected: AccessorySelection? = null
    private var description: AccessoryDescription? = null
    init { require(initialEpoch >= 0) }

    fun select(snapshot: List<AccessoryDescription>): AccessorySelection {
        invalidate()
        check(epoch < Long.MAX_VALUE) { "Accessory selection epoch exhausted" }
        epoch++
        val value = resolver.choose(snapshot)
        description = value
        return AccessorySelection(epoch, value.digest).also { selected = it }
    }
    fun current(selection: AccessorySelection): Boolean = selected == selection
    fun confirm(selection: AccessorySelection, snapshot: List<AccessoryDescription>, permitted: Boolean): AccessoryDescription? {
        if (!current(selection)) return null
        try {
            val value = resolver.resolve(selection, snapshot)
            if (!permitted || value != description) { invalidate(); return null }
            return value
        } catch (failure: Exception) { invalidate(); throw failure }
    }
    fun invalidate() { selected = null; description = null }
}
