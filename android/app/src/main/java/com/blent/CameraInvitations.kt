package com.blent

import kotlinx.coroutines.flow.MutableStateFlow

/** One authoritative memory-only ADB endpoint. Native owners settle separately. */
internal object CameraInvitations {
    val endpoint = MutableStateFlow<CameraEndpoint?>(null)

    fun stop(request: CameraStop): Boolean {
        val original = endpoint.value ?: return false
        if (!request.matches(original)) return false
        return endpoint.compareAndSet(original, null)
    }
}
