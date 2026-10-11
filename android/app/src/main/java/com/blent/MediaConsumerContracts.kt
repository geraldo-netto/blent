package com.blent

/** One possible native request, retained separately from its route's visibility.
 * Only actual retirement/no-start releases it. Implementations must bind current
 * to the original owner, never a mutable latest connection or matching counter. */
internal interface MediaConsumerClaim {
    fun current(): Boolean
    fun retired(proof: ResourceRetirement)
}

/** Focused portable controls. Retain refuses a closed target; stop requests only
 * this resource's closure and cannot substitute for actual native completion. */
internal interface MediaConsumerAuthority {
    fun retain(): MediaConsumerClaim?
    fun stop(): Boolean
}

/** Endpoint metadata stays resource-owned; authority remains independent from
 * transport credentials, native processes, permission policy and UI. */
internal interface MediaConsumerTarget<E> {
    val endpoint: E
    val authority: MediaConsumerAuthority
}
