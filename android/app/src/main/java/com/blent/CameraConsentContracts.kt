package com.blent

/** owner is an opaque original lifetime, stable across its offer/permission/route
 * publications, never a numeric lease or DTO equality. Contains no credential. */
internal class CameraAccessOffer(
    override val owner: Any,
    val lens: CameraLens,
    override val backgroundRequested: Boolean,
    override val permissionRequired: Boolean,
    override val admitted: Boolean,
    override val authority: MediaAccessAuthority,
) : MediaAccessOffer {
    init { require(!permissionRequired || !admitted) }
}
