package com.blent

import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.asStateFlow

/** Serialized portable consent policy on the camera owner's dispatcher. Starts
 * require explicit local intent; original lifetime identity carries intent across
 * offer/permission/admission publications, never across a replacement. Native
 * capture and SDK work belong to separate adapters. */
internal class CameraConsentPolicy(
    private val permission: () -> Boolean,
    private val prompts: CameraPermissionPrompts,
) {
    private val mutable = MutableStateFlow(CameraConsentView())
    val state = mutable.asStateFlow()
    private var pending: CameraPermissionAction? = null

    fun observe(value: CameraAccessOffer?) {
        val previous = mutable.value
        if (previous.offer === value) return
        cancelPermission()
        val sameOwner = previous.offer != null && previous.offer.owner === value?.owner
        mutable.value = if (sameOwner) previous.copy(offer = value)
            else CameraConsentView(offer = value, foreground = previous.foreground)
        if (!sameOwner && previous.enabled) previous.offer?.let { update(it, context(enabled = false)) }
        if (mutable.value.enabled) maintain()
    }

    fun start(offer: CameraAccessOffer): Boolean {
        if (!current(offer) || !mutable.value.foreground) return false
        mutable.value = mutable.value.copy(enabled = true)
        return maintain()
    }

    fun stop(offer: CameraAccessOffer): Boolean {
        if (!current(offer)) return false
        disable(offer.owner)
        return update(offer, context(enabled = false))
    }

    fun background(offer: CameraAccessOffer, allowed: Boolean): Boolean {
        if (!current(offer)) return false
        mutable.value = mutable.value.copy(backgroundAllowed = allowed)
        if (!mutable.value.enabled) return true
        if (!mutable.value.foreground && !keepBackground()) return stop(offer)
        return maintain()
    }

    fun foreground(value: Boolean) {
        mutable.value = mutable.value.copy(foreground = value)
        val offer = mutable.value.offer ?: return
        if (!mutable.value.enabled) return
        if (!value && !keepBackground()) stop(offer) else maintain()
    }

    fun permissionChanged() { if (mutable.value.enabled) maintain() }

    private fun keepBackground(): Boolean = mutable.value.backgroundAllowed &&
        mutable.value.offer?.backgroundRequested == true && permitted()

    private fun permitted(): Boolean = try { permission() } catch (_: Exception) { false }

    private fun context(enabled: Boolean = mutable.value.enabled): CameraLocalConsent =
        CameraLocalConsent(mutable.value.foreground, permitted(), enabled, mutable.value.backgroundAllowed)

    private fun current(offer: CameraAccessOffer): Boolean =
        mutable.value.offer === offer && live(offer)

    private fun live(offer: CameraAccessOffer): Boolean = try { offer.authority.current() }
        catch (_: Exception) { false }

    private fun update(offer: CameraAccessOffer, context: CameraLocalConsent): Boolean = try {
        live(offer) && offer.authority.update(context)
    } catch (_: Exception) { false }

    private fun maintain(): Boolean {
        val offer = mutable.value.offer ?: return false
        val context = context()
        if (!update(offer, context)) { disable(offer.owner); return false }
        if (mutable.value.offer !== offer) return true // A synchronous publication already handled it.
        if (requiresClose(offer, context)) closePermissionLoss(offer, context)
        else if (offer.permissionRequired && !context.permission && context.foreground) requestPermission(offer)
        return true
    }

    private fun requiresClose(offer: CameraAccessOffer, context: CameraLocalConsent): Boolean =
        !context.permission && (offer.admitted || !context.foreground)

    private fun closePermissionLoss(offer: CameraAccessOffer, context: CameraLocalConsent) {
        disable(offer.owner)
        // Preparing consent can still be waiting for permission; explicitly
        // close it when no foreground exists to complete that original prompt.
        if (!offer.admitted) update(offer, context.copy(enabled = false))
    }

    private fun requestPermission(offer: CameraAccessOffer) {
        val previous = pending
        if (previous != null && prompts.owns(previous)) return
        pending = null
        val action = object : CameraPermissionAction {
            override fun current(): Boolean = this@CameraConsentPolicy.current(offer) &&
                mutable.value.foreground && mutable.value.enabled
            override fun complete(granted: Boolean) {
                pending = null
                permissionResult(offer, granted)
            }
        }
        pending = action
        if (!prompts.request(action)) {
            if (pending === action) pending = null
            permissionResult(offer, false)
        }
    }

    private fun permissionResult(offer: CameraAccessOffer, granted: Boolean) {
        if (!current(offer)) return
        val context = context()
        val accepted = try { offer.authority.permissionResult(granted, context) }
            catch (_: Exception) { false }
        if (!granted || !context.permission || !accepted) disable(offer.owner)
    }

    private fun cancelPermission() {
        pending?.let { prompts.cancel(it) }
        pending = null
    }

    private fun disable(owner: Any) {
        if (mutable.value.offer?.owner !== owner) return
        cancelPermission()
        mutable.value = mutable.value.copy(enabled = false)
    }

    fun clear() { observe(null) }
}
