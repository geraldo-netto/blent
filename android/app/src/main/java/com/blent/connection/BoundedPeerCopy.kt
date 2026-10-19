package com.blent.connection

/** Synchronous borrowed input; admit count before allocation or element access.
 * At most maximum immutable peer references escape in an independently owned
 * list. No iterator, caller list or callback is retained. Invalid/failed reads
 * publish no result and create no native obligation. */
internal fun boundedPeerCopy(peers: List<ConnectionPeer>, maximum: Int): List<ConnectionPeer> {
    require(maximum in 0..1024)
    val count = peers.size
    require(count in 0..maximum)
    return List(count) { peers[it] }
}
