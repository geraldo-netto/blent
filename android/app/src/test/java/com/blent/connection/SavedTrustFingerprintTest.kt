package com.blent.connection

import org.junit.Assert.*
import org.junit.Test

class SavedTrustFingerprintTest {
    @Test fun t1151SharedFingerprintUsesExactImmutableCertificateBytes() {
        val bytes = javaClass.classLoader!!.getResourceAsStream("connection-tls/tablet.der")!!.use { it.readBytes() }
        val identity = ConnectionIdentity(bytes)
        bytes.fill(0)
        identity.certificate().fill(1)
        assertEquals("931f7dfd81d060aa0364636f80958d12b384d41a40be5aa658e27f7f7c4b3b35", identity.fingerprint().fingerprint)
        assertEquals(identity.fingerprint(), identity.fingerprint())
    }
}
