package com.blent.connection

import com.blent.*
import kotlinx.coroutines.flow.*

/** One original peer/lease and exact publication authority. Typed adapters select
 * metadata; consent never depends on a concrete connection worker or OS device. */
internal class ConnectionMediaAccess<C : ConnectionMediaChoice, O : MediaAccessOffer, E, T : MediaConsumerTarget<E>>(
    private val binding: ConnectionBinding,
    private val choice: (ConnectionView) -> C?,
    targets: StateFlow<T?>,
    private val endpoint: (C) -> E?,
    private val offer: (Any, C, MediaAccessAuthority) -> O,
    private val update: (C, MediaLocalConsent) -> Boolean,
    private val permission: (C, Boolean, MediaLocalConsent) -> Boolean,
) {
    private class Original(val peer: ConnectionChoice, val lease: ConnectionResourceLease) { val identity = Any() }
    private class Publication<C, O, E>(val choice: C, val offer: O, val endpoint: E?)
    private var original: Original? = null
    private var published: Publication<C, O, E>? = null
    private val targets = targets
    val offers: Flow<O?> = binding.state.map { project(binding.state.value) }
    val captures: Flow<MediaConsumerPublication<T>?> = combine(binding.state, targets) { _, _ -> capture() }

    fun capture(): MediaConsumerPublication<T>? {
        val offer = project() ?: return null
        if (!offer.admitted) return null
        val choice = currentChoice(binding.state.value) ?: return null
        val publication = published ?: return null
        val target = targets.value ?: return null
        if (publication.choice !== choice || target.endpoint != publication.endpoint) return null
        return MediaConsumerPublication(offer.owner, target)
    }
    fun project(view: ConnectionView = binding.state.value): O? {
        val choice = currentChoice(view)
        if (choice == null) { clear(); return null }
        val peer = checkNotNull(view.peer)
        val previous = published
        if (previous?.choice === choice && original?.peer === peer) return previous.offer
        val owner = original(peer, choice.lease)
        val publication = Publication(choice, offer(owner.identity, choice, authority(owner, choice)), endpoint(choice))
        published = publication
        return publication.offer
    }
    private fun clear() { original = null; published = null }
    private fun original(peer: ConnectionChoice, lease: ConnectionResourceLease): Original {
        val previous = original
        if (previous?.peer === peer && previous.lease == lease) return previous
        return Original(peer, lease).also { original = it }
    }
    private fun currentChoice(view: ConnectionView): C? {
        if (!view.active || !view.authenticated || view.peer == null) return null
        val selected = choice(view)
        if (selected?.lease?.connection != view.peer.ticket) return null
        return selected
    }
    private fun current(owner: Original, choice: C): Boolean =
        binding.state.value.peer === owner.peer && currentChoice(binding.state.value) === choice
    private fun authority(owner: Original, choice: C) = object : MediaAccessAuthority {
        override fun current() = this@ConnectionMediaAccess.current(owner, choice)
        override fun update(context: MediaLocalConsent) = current() && update(choice, context)
        override fun permissionResult(granted: Boolean, context: MediaLocalConsent) =
            current() && permission(choice, granted, context)
    }
}
