package com.blent

/** One local permission request lifetime. Equal/reused endpoint data never
 * authorizes an SDK callback belonging to a previous invitation. */
internal class AudioCapturePermission(val endpoint: AudioEndpoint)


/** Independent original audio offer. Contains no transport/credential/native
 * handle. Speaker playback never requires microphone SDK permission. */
internal class AudioAccessOffer(
    override val owner: Any,
    val direction: Int,
    override val backgroundRequested: Boolean,
    override val permissionRequired: Boolean,
    override val admitted: Boolean,
    override val authority: MediaAccessAuthority,
) : MediaAccessOffer {
    init {
        require(direction in 1..2)
        require(!permissionRequired || !admitted)
        require(direction == 1 || !permissionRequired)
    }
}
