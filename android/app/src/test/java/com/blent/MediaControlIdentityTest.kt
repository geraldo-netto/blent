package com.blent

import org.junit.Assert.*
import org.junit.Test

class MediaControlIdentityTest {
    @Test fun t1247SameOriginalOwnerHasStableKeyEqualityAndHash() {
        for (owner in listOf(Any(), 0, Int.MAX_VALUE, "original", emptyList<Int>())) {
            val first = MediaControlIdentity(owner)
            val second = MediaControlIdentity(owner)
            assertEquals(first, first)
            assertEquals(first, second)
            assertEquals(first.hashCode(), second.hashCode())
            assertEquals(System.identityHashCode(owner), first.hashCode())
        }
    }

    @Test fun t1247EqualValueOwnersCannotMergeControlsAndOtherKeyTypesReject() {
        val first = listOf("same")
        val second = listOf("same")
        assertEquals(first, second); assertNotSame(first, second)
        val key = MediaControlIdentity(first)
        assertNotEquals(key, MediaControlIdentity(second))
        assertFalse(key.equals(null))
        assertFalse(key.equals(first))
        assertNotEquals(key, MediaControlIdentity(Any()))
    }
}
