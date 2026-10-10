package com.blent.connection

/** Native permission boundary. All methods except failure/retired can block;
 * the admission owner invokes them on its native worker or separate closer.
 * close may race start/request. A late-created native handle remains owned. */
internal interface AccessoryAdmissionBackend : ConnectionSetupLifetime {
    fun start(dispatch: (() -> Unit) -> Unit,
        report: (AccessorySelection, List<AccessoryDescription>, Boolean) -> Unit,
        detached: () -> Unit)
    fun snapshot(): List<AccessoryDescription>
    fun permitted(selection: AccessorySelection): Boolean
    fun request(selection: AccessorySelection)
    fun failure(): Exception?
}

/** USB permission grants a local candidate only. TLS and media consent are separate.
 * Methods are prompt. Caller supplies an independent advance timer and must retain
 * this owner until retired, even after timeout or terminationRequired. */
internal interface AccessoryAdmission {
    fun advance()
    fun takeReady(): AccessorySelection?
    fun current(selection: AccessorySelection): Boolean
    fun stop()
    fun retired(): Boolean
    fun failure(): Exception?
    fun terminationRequired(): Boolean
}
