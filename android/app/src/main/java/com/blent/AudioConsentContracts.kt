package com.blent

/** One local permission request lifetime. Equal/reused endpoint data never
 * authorizes an SDK callback belonging to a previous invitation. */
internal class AudioCapturePermission(val endpoint: AudioEndpoint)
