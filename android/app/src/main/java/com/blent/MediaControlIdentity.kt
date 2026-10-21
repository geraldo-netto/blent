package com.blent

/** Compose retains controls within one original offer lifetime. Ordinary value
 * equality cannot merge different owners. This key borrows its owner only for
 * the containing composition; it owns no native resource, queue or permission. */
internal class MediaControlIdentity(private val owner: Any) {
    override fun equals(other: Any?): Boolean = other is MediaControlIdentity && owner === other.owner
    override fun hashCode(): Int = System.identityHashCode(owner)
}
