package com.blent

/** One original legacy permission request. Reference identity, not matching
 * endpoint/lens values, determines which SDK result may finish it. */
internal class CameraCapturePermission(val endpoint: CameraEndpoint, val lens: CameraLens)
