package com.blent.connection

import org.junit.Assert.*
import org.junit.Test
import java.util.Random

class AccessorySelectionTest {
    private val profile = AccessoryProfile("Blent", "Blent USB Probe", "1")
    private fun device(serial: String? = "") = AccessoryDescription("Blent", "Blent USB Probe", "T846", "1", null, serial)
    private fun unsupported() = device().copy(model = "other")

    @Test fun t887SixFieldLengthPrefixedDigestMatchesIndependentGolden() {
        val value = device()
        assertEquals("T846", value.description); assertNull(value.uri); assertEquals("", value.serial)
        assertEquals("Blent", profile.manufacturer); assertEquals("Blent USB Probe", profile.model); assertEquals("1", profile.version)
        assertEquals("e070ce345bab16b573fb380ef381b3331b1cd441d2065fb059bdb03aeed4200c", value.digest)
        assertEquals(value.digest, value.copy().digest)
        assertNotEquals(value.digest, device(null).digest)
        assertNotEquals(value.digest, value.copy(uri = "").digest)
        assertNotEquals(value.digest, value.copy(description = "different").digest)
        assertNotEquals(value.digest, value.copy(version = "2").digest)
        assertNotEquals(value.digest, value.copy(manufacturer = "other").digest)
        assertNotEquals(value.digest, value.copy(model = "other").digest)
        assertTrue(profile.matches(value)); assertFalse(profile.matches(value.copy(version = null)))
        assertFalse(profile.matches(unsupported())); assertFalse(profile.matches(value.copy(manufacturer = null)))
        assertEquals(64, AccessoryDescription(null, null, null, null, null, null).digest.length)
    }

    @Test fun t887MetadataBoundsRejectMalformedUnicodeWithoutReplacementCollisions() {
        assertEquals(64, device("a".repeat(512)).digest.length)
        assertEquals(64, device("é".repeat(256)).digest.length)
        assertEquals(64, device("😀".repeat(128)).digest.length)
        listOf("a".repeat(513), "é".repeat(257), "\uD800", "\uDC00").forEach {
            assertThrows(IllegalArgumentException::class.java) { device(it) }
        }
        assertThrows(IllegalArgumentException::class.java) { AccessoryMetadata.digest(listOf(null)) }
        assertThrows(IllegalArgumentException::class.java) { profile.copy(manufacturer = "") }
        assertThrows(IllegalArgumentException::class.java) { profile.copy(model = "") }
        assertThrows(IllegalArgumentException::class.java) { profile.copy(version = "") }
        assertThrows(IllegalArgumentException::class.java) { profile.copy(version = "\uD800") }
    }

    @Test fun t887CanonicalSelectionRoundTripsCounterBoundaries() {
        for (epoch in listOf(1L, 2, 999, Long.MAX_VALUE)) {
            val token = AccessorySelection(epoch, device().digest)
            assertEquals(token, AccessorySelection.parse(token.encode()))
            assertTrue(token.encode().length <= 84)
        }
        assertThrows(IllegalArgumentException::class.java) { AccessorySelection(0, device().digest) }
        assertThrows(IllegalArgumentException::class.java) { AccessorySelection(-1, device().digest) }
    }

    @Test fun t887BoundedMalformedTokensNeverAcquireSelection() {
        val digest = device().digest
        val invalid = listOf("", "1", "0:$digest", "-1:$digest", "01:$digest", "+1:$digest",
            "1:$digest ", "1:${digest.uppercase()}", "9223372036854775808:$digest", "1;$digest", ":$digest")
        invalid.forEach { assertThrows(IllegalArgumentException::class.java) { AccessorySelection.parse(it) } }
        val random = Random(887)
        repeat(512) {
            val value = "1:$digest".toCharArray()
            value[2 + random.nextInt(64)] = ('g'.code + random.nextInt(20)).toChar()
            assertThrows(IllegalArgumentException::class.java) { AccessorySelection.parse(String(value)) }
        }
    }

    @Test fun t887SelectionNeedsUniqueSupportedSnapshotAndFreshPermission() {
        val owner = AccessorySelectionOwner(profile)
        val device = device(); val snapshot = listOf(unsupported(), device)
        val selected = owner.select(snapshot)
        assertEquals(1L, selected.epoch); assertTrue(owner.current(selected))
        assertEquals(device, owner.confirm(selected, snapshot, true))
        assertNull(owner.confirm(selected, snapshot, false)); assertFalse(owner.current(selected))
        val replacement = owner.select(snapshot)
        assertEquals(2L, replacement.epoch); assertTrue(owner.current(replacement))
        assertNull(owner.confirm(selected, emptyList(), false)); assertTrue(owner.current(replacement))
        owner.invalidate(); owner.invalidate(); assertFalse(owner.current(replacement))
    }

    @Test fun t887FailedReplacementAndChangedSnapshotInvalidatePriorToken() {
        val owner = AccessorySelectionOwner(profile)
        val selected = owner.select(listOf(device()))
        assertThrows(IllegalStateException::class.java) { owner.select(emptyList()) }
        assertFalse(owner.current(selected))
        val replacement = owner.select(listOf(device()))
        assertEquals(3L, replacement.epoch)
        assertThrows(IllegalStateException::class.java) { owner.confirm(replacement, listOf(device("changed")), true) }
        assertFalse(owner.current(replacement))
    }

    @Test fun t887DuplicateAndAmbiguousInventoriesNeverChooseArbitraryDevice() {
        val resolver = AccessoryResolver(profile)
        val first = device(); val second = device("second")
        assertThrows(IllegalStateException::class.java) { resolver.choose(listOf(first, second)) }
        assertThrows(IllegalStateException::class.java) { resolver.choose(listOf(first, first)) }
        assertThrows(IllegalStateException::class.java) { resolver.choose(listOf(unsupported())) }
        val selected = AccessorySelection(1, first.digest)
        assertEquals(first, resolver.resolve(selected, listOf(unsupported(), first, second)))
        assertThrows(IllegalStateException::class.java) { resolver.resolve(selected, listOf(first, first)) }
        assertThrows(IllegalStateException::class.java) { resolver.resolve(selected, emptyList()) }
        val other = unsupported()
        assertThrows(IllegalStateException::class.java) { resolver.resolve(AccessorySelection(1, other.digest), listOf(other)) }
    }

    @Test fun t887InventoryCapacityIsCheckedBeforeReadingElements() {
        val tooLarge = object : AbstractList<AccessoryDescription>() {
            override val size = 1025
            override fun get(index: Int): AccessoryDescription = error("Unbounded snapshot was traversed")
        }
        val resolver = AccessoryResolver(profile, 1024)
        assertThrows(IllegalArgumentException::class.java) { resolver.choose(tooLarge) }
        assertThrows(IllegalArgumentException::class.java) { resolver.resolve(AccessorySelection(1, device().digest), tooLarge) }
        assertEquals(device(), resolver.choose(List(1023) { unsupported() } + device()))
        assertThrows(IllegalArgumentException::class.java) { AccessoryResolver(profile, 0) }
        assertThrows(IllegalArgumentException::class.java) { AccessoryResolver(profile, 1025) }
        assertThrows(IllegalArgumentException::class.java) { AccessorySelectionOwner(profile, initialEpoch = -1) }
    }

    @Test fun t887EpochExhaustionInvalidatesWithoutWrappingOrReusingToken() {
        val owner = AccessorySelectionOwner(profile, initialEpoch = Long.MAX_VALUE - 1)
        val last = owner.select(listOf(device()))
        assertEquals(Long.MAX_VALUE, last.epoch)
        assertThrows(IllegalStateException::class.java) { owner.select(listOf(device())) }
        assertFalse(owner.current(last)); assertNull(owner.confirm(last, listOf(device()), true))
        assertThrows(IllegalStateException::class.java) { owner.select(emptyList()) }
    }

    @Test fun t887SeededLifecycleMutationsCannotReviveStaleTickets() {
        val random = Random(888)
        val owner = AccessorySelectionOwner(profile, 1)
        val resolver = AccessoryResolver(profile, 1)
        var previous: AccessorySelection? = null
        repeat(256) { index ->
            val value = device(index.toString())
            val selected = owner.select(listOf(value))
            previous?.let { assertFalse(owner.current(it)); assertNull(owner.confirm(it, listOf(value), true)) }
            assertEquals(value, resolver.resolve(AccessorySelection.parse(selected.encode()), listOf(value)))
            val permission = random.nextBoolean()
            assertEquals(if (permission) value else null, owner.confirm(selected, listOf(value), permission))
            owner.invalidate(); assertFalse(owner.current(selected)); previous = selected
        }
    }
}
