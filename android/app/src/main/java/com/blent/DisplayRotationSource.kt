package com.blent

/** One process owner, serialized with camera selection on its UI dispatcher.
 * start/stop owns exactly one observation. Stop fences queued notifications;
 * failed registration/unregistration remains owned until stop succeeds. */
internal interface DisplayRotationSource {
    /** Default display quarter-turns, 0..3; absent display uses the neutral 0. */
    fun current(): Int
    fun start(changed: () -> Unit)
    fun stop()
}
