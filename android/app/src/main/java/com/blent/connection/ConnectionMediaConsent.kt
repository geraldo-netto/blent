package com.blent.connection

import com.blent.MediaLocalConsent

/** Independent original resource proof, supplied by the retained native boundary.
 * A parsed lease, pairing approval or SDK result cannot implement this proof. */
internal interface ConnectionMediaConsentAuthority {
    fun owns(lease: ConnectionResourceLease): Boolean
    fun admitted(lease: ConnectionResourceLease): Boolean
}
internal sealed interface ConnectionMediaConsentAction<out P> {
    data class Permission<P>(val request: P) : ConnectionMediaConsentAction<P>
    data object Prepare : ConnectionMediaConsentAction<Nothing>
    data class Reject(val outcome: ConnectionManagement.Outcome) : ConnectionMediaConsentAction<Nothing>
    data class Cancel(val reason: ConnectionManagement.Reason) : ConnectionMediaConsentAction<Nothing>
}

/** Serialized consent for one original media lease. Typed adapters validate their
 * profile and create opaque prompt identity; this policy owns neither credentials,
 * OS permissions, devices nor retirement. Closing never proves native completion. */
internal class ConnectionMediaConsent<P : Any>(
    private val lease: ConnectionResourceLease,
    private val backgroundRequested: Boolean,
    private val permissionPrompt: Boolean,
    private val authority: ConnectionMediaConsentAuthority,
    private val prompt: (ConnectionResourceLease) -> P,
) {
    init {
        require(lease.key.resource != ConnectionManagement.Resource.DISPLAY)
        ConnectionLocalLease.validate(lease, lease.key.resource)
    }
    private var request: P? = null
    private var opening = false
    private var closed = false
    fun pending(): P? = request
    fun closing(): Boolean = closed
    fun update(choice: MediaLocalConsent): ConnectionMediaConsentAction<P>? {
        if (closed) return null
        if (!authority.owns(lease)) return connectionLost()
        if (!choice.enabled) return stop()
        return if (opening) maintain(choice) else prepare(choice)
    }
    private fun allowed(choice: MediaLocalConsent): Boolean = choice.enabled && choice.permission &&
        (choice.foreground || (backgroundRequested && choice.backgroundAllowed))
    private fun maintain(choice: MediaLocalConsent): ConnectionMediaConsentAction<P>? {
        if (!choice.permission) return cancel(ConnectionManagement.Reason.PERMISSION_REVOKED)
        if (!allowed(choice)) return stop()
        return null
    }
    private fun prepare(choice: MediaLocalConsent): ConnectionMediaConsentAction<P>? {
        if (allowed(choice)) {
            opening = true
            request = null
            return ConnectionMediaConsentAction.Prepare
        }
        if (!permissionPrompt || !choice.foreground || request != null) return null
        return prompt(lease).let { request = it; ConnectionMediaConsentAction.Permission(it) }
    }
    fun permissionResult(value: P, granted: Boolean, choice: MediaLocalConsent): ConnectionMediaConsentAction<P>? {
        if (closed || value !== request) return null
        request = null
        if (!authority.owns(lease)) return connectionLost()
        if (!choice.foreground || !choice.enabled) return stop()
        if (!granted || !choice.permission) return reject(ConnectionManagement.Outcome.PERMISSION_DENIED)
        return update(choice)
    }
    fun preparationAllowed(value: ConnectionResourceLease, choice: MediaLocalConsent): Boolean =
        !closed && opening && value == lease && allowed(choice) && authority.owns(value)
    fun captureAllowed(value: ConnectionResourceLease, choice: MediaLocalConsent): Boolean =
        preparationAllowed(value, choice) && authority.admitted(value)
    fun stop(): ConnectionMediaConsentAction<P>? {
        if (closed) return null
        return if (opening) cancel(ConnectionManagement.Reason.REQUESTED) else reject(ConnectionManagement.Outcome.UNAVAILABLE)
    }
    fun connectionLost(): ConnectionMediaConsentAction<P>? =
        if (closed) null else cancel(ConnectionManagement.Reason.CONNECTION_LOST)
    private fun cancel(reason: ConnectionManagement.Reason): ConnectionMediaConsentAction<P> {
        close(); return ConnectionMediaConsentAction.Cancel(reason)
    }
    private fun reject(outcome: ConnectionManagement.Outcome): ConnectionMediaConsentAction<P> {
        close(); return ConnectionMediaConsentAction.Reject(outcome)
    }
    private fun close() { closed = true; request = null }
}
