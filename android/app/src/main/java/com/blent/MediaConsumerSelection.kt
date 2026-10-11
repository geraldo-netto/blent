package com.blent

/** Local consent and original transport authority must both remain current at
 * acquisition. A stable wrapper avoids restarting media on unrelated UI wakes. */
internal class MediaConsumerSelection<O : MediaAccessOffer, E, T : MediaConsumerTarget<E>>(
    private val consent: () -> MediaConsentView<O>,
    private val permission: () -> Boolean,
    private val target: (E, MediaConsumerAuthority) -> T,
) {
    private var publication: MediaConsumerPublication<T>? = null
    private var selected: T? = null
    @Volatile private var identity: Any? = null

    fun project(value: MediaConsumerPublication<T>?): T? {
        if (value == null || !permitted(value.owner)) {
            identity = null; publication = null; selected = null
            return null
        }
        val previous = publication
        if (previous?.owner === value.owner && previous.target === value.target) return selected
        publication = value
        val original = Any().also { identity = it }
        return target(value.target.endpoint, authority(value, original)).also { selected = it }
    }

    private fun local(view: MediaConsentView<O>, offer: O): Boolean =
        view.enabled && offer.admitted &&
            (view.foreground || (view.backgroundAllowed && offer.backgroundRequested))

    private fun permitted(owner: Any): Boolean = try {
        val view = consent()
        val offer = view.offer
        offer?.owner === owner && local(view, checkNotNull(offer)) &&
            permission() && offer.authority.current()
    } catch (_: Exception) { false }

    private fun current(owner: Any, original: Any) = identity === original && permitted(owner)

    private fun authority(value: MediaConsumerPublication<T>, identity: Any) = object : MediaConsumerAuthority {
        override fun retain(): MediaConsumerClaim? {
            if (!current(value.owner, identity)) return null
            val original = value.target.authority.retain() ?: return null
            return claim(value.owner, identity, original)
        }
        override fun stop() = value.target.authority.stop()
    }

    private fun claim(owner: Any, identity: Any, original: MediaConsumerClaim) = object : MediaConsumerClaim {
        override fun current() = current(owner, identity) && original.current()
        override fun retired(proof: ResourceRetirement) = original.retired(proof)
    }
}
