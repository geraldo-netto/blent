package com.blent

/** Immutable local registration snapshot, not remote authority or backend support.
 * Ports are assigned by a local adapter; management messages cannot choose them.
 * Null credentials preserve the existing ADB startup handshake. Credential
 * acceptance remains with token-delivery and private worker IPC boundaries. */
internal data class DisplayRoutes(val revision: Long, val token: String?, val videoPort: Int, val controlPort: Int) {
    init {
        require(revision > 0)
        require(videoPort in 1..65535 && controlPort in 1..65535 && videoPort != controlPort)
    }
    override fun toString() = "DisplayRoutes(revision=$revision, videoPort=$videoPort, controlPort=$controlPort, credential=${token != null})"
}

/** Optional ADB composition adapter. Calling create is explicit route selection. */
internal object AdbDisplayRoutes {
    private const val VIDEO_PORT = 8890
    private const val CONTROL_PORT = 8891
    val controlUrl get() = "ws://127.0.0.1:$CONTROL_PORT"
    fun create(token: String? = null, revision: Long = 1) = DisplayRoutes(revision, token, VIDEO_PORT, CONTROL_PORT)
}
