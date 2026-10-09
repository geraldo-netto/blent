package com.blent

import org.junit.Assert.*

/** T798 fixture barrier: UI Stop is asynchronous; assertions about completed
 * native destruction and fixture reuse must await actual owner completion. */
internal fun awaitDecoderRetirement(decoder: DecoderSession? = null) {
    val field = CodecLifetime::class.java.getDeclaredField("retiringOwners").apply { isAccessible = true }
    @Suppress("UNCHECKED_CAST")
    val owners = (field.get(null) as Set<CodecLifetime>).toMutableSet()
    if (decoder != null) {
        val retired = DecoderSession::class.java.getDeclaredField("retiring").apply { isAccessible = true }
        (retired.get(decoder) as CodecLifetime?)?.let { owners.add(it) }
    }
    for (owner in owners) {
        owner.awaitRetirement(2000)
        assertTrue("T798 fixture native retirement did not finish", owner.finished)
    }
    assertFalse("T798 fixture left global replacement admission closed", CodecLifetime.retirementPending())
}
