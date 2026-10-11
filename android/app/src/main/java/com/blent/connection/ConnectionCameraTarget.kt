package com.blent.connection

import com.blent.CameraEndpoint
import com.blent.CameraLens

/** Boundary projection only; metadata carries neither current consent nor a
 * native grant. The independent capture contract retains original authority. */
internal object ConnectionCameraTarget {
    fun endpoint(route: LocalCameraRoute): CameraEndpoint {
        val profile = route.profile
        val token = route.credential.bytes().joinToString("") { "%02x".format(it.toInt() and 255) }
        return CameraEndpoint(token, route.port, profile.width, profile.height,
            profile.fps, profile.bitrate, CameraLens.entries[profile.lens], profile.background,
            profile.freshnessMs, profile.adaptive, profile.minBitrate)
    }
}
