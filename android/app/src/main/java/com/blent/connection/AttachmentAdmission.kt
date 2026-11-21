package com.blent.connection

/** Attachment admission is independent of Activity intents and peer authentication.
 * Manual starts consume the current automatic opportunity but need no fresh event. */
internal interface AttachmentAdmission {
    suspend fun take(): Boolean
    /** Current supported accessory observation; never consumes an attach event. */
    suspend fun present(): Boolean
    fun consume()
}

/** Process-lifetime freshness owner. Only the native adapter may renew an epoch.
 * Metadata equality never renews it. Identity objects cannot wrap or be reused. */
internal class AttachmentEpoch {
    private var identity = Any()
    private var consumed = false
    @Synchronized fun candidate(): Any? = identity.takeUnless { consumed }
    @Synchronized fun observation(): Any = identity
    @Synchronized fun unchanged(observed: Any): Boolean = observed === identity
    @Synchronized fun claim(candidate: Any): Boolean {
        if (candidate !== identity || consumed) return false
        consumed = true
        return true
    }
    @Synchronized fun consume() { identity = Any(); consumed = true }
    @Synchronized fun detached() { identity = Any(); consumed = false }
}
