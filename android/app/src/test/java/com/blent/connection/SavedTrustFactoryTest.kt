package com.blent.connection

import android.content.ContextWrapper
import java.io.File
import java.security.KeyStore
import java.security.Provider
import java.security.Security
import java.security.cert.Certificate
import java.security.cert.CertificateFactory
import kotlinx.coroutines.runBlocking
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.RuntimeEnvironment
import org.robolectric.annotation.Config

/** Private entry/signing access is deliberately forbidden by the test provider. */
class SavedTrustReadOnlyStoreFixture : ConnectionFactoryKeyStoreFixture() {
    override fun engineGetEntry(alias: String?, protection: KeyStore.ProtectionParameter?): KeyStore.Entry =
        error("T1151 browsing requested a private entry")
    override fun engineGetCertificate(alias: String?): Certificate {
        check(alias == AndroidConnectionIdentity.ALIAS)
        return javaClass.classLoader!!.getResourceAsStream("connection-tls/tablet.der")!!.use {
            CertificateFactory.getInstance("X.509").generateCertificate(it)
        }
    }
}

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [27, 34])
class SavedTrustFactoryTest {
    @get:Rule val temporary = TemporaryFolder()
    private fun context() = object : ContextWrapper(RuntimeEnvironment.getApplication()) {
        override fun getNoBackupFilesDir(): File = temporary.root
        override fun getSystemService(name: String): Any? = error("T1151 offline browsing accessed native service $name")
    }
    private fun provider() = object : Provider("BlentSavedTrustFixture", 1.0, "Synthetic read-only certificate") {}.apply {
        put("KeyStore.AndroidKeyStore", SavedTrustReadOnlyStoreFixture::class.java.name)
    }
    private fun peer() = ConnectionPeer.parse("a".repeat(64))

    @Test fun t1151CreationAndUnstartedRetirementHaveNoNativeEffects() = runBlocking<Unit> {
        val factory = AndroidSavedTrustFactory({ error("T1151 unexpected context access") })
        factory.create(SavedTrustRequest.List) { error("T1151 unexpected event") }.retire()
        assertTrue(temporary.root.listFiles()!!.isEmpty())
    }

    @Test fun t1151ExistingPublicCertificateDoesNotReadPrivateEntryOrOpenUsb() = runBlocking<Unit> {
        val provider = provider()
        check(Security.insertProviderAt(provider, 1) == 1)
        try {
            val events = mutableListOf<ConnectionWorkerEvent>()
            val factory = AndroidSavedTrustFactory(::context)
            val worker = factory.create(SavedTrustRequest.List, events::add)
            try { worker.execute() } finally { worker.retire() }
            val page = (events.single() as ConnectionWorkerEvent.SavedComputers).page
            assertEquals(expectedFingerprint, page.local!!.fingerprint)
            assertEquals(0, page.total); assertEquals(0L, page.revision)
            val directory = AndroidConnectionFactory(::context).directory()
            assertFalse(directory.resolve("trust.bin").toFile().exists())
            AndroidPeerTrustStore.open(directory).close()
        } finally { Security.removeProvider(provider.name) }
    }

    @Test fun t1151AbsentPublicIdentityAndFactoryFailureDoNotInventKeys() = runBlocking<Unit> {
        assertNull(currentSavedTrustIdentity { KeyStore.getInstance("PKCS12") })
        assertThrows(IllegalStateException::class.java) { currentSavedTrustIdentity { error("provider unavailable") } }
        val events = mutableListOf<ConnectionWorkerEvent>()
        val empty = AndroidSavedTrustFactory(::context, { null }).create(SavedTrustRequest.List, events::add)
        try { empty.execute() } finally { empty.retire() }
        assertNull((events.single() as ConnectionWorkerEvent.SavedComputers).page.local)
        val failure = AndroidSavedTrustFactory(::context, { error("T1151 provider unavailable") })
            .create(SavedTrustRequest.List) { error("T1151 failed identity published results") }
        assertTrue(runCatching { failure.execute() }.isFailure)
        assertTrue(runCatching { failure.retire() }.isFailure)
        AndroidPeerTrustStore.open(AndroidConnectionFactory(::context).directory()).close()
    }

    @Test fun t1151ExistingConnectionOwnershipRejectsMaintenanceBeforeReadingIdentity() = runBlocking<Unit> {
        val directory = AndroidConnectionFactory(::context).directory()
        var reads = 0
        AndroidPeerTrustStore.open(directory).use {
            val worker = AndroidSavedTrustFactory(::context, { reads++; peer() }).create(SavedTrustRequest.List) {}
            assertTrue(runCatching { worker.execute() }.isFailure)
            assertTrue(runCatching { worker.retire() }.isFailure)
            assertEquals(0, reads)
        }
        AndroidPeerTrustStore.open(directory).close()
    }

    companion object { const val expectedFingerprint = "931f7dfd81d060aa0364636f80958d12b384d41a40be5aa658e27f7f7c4b3b35" }
}
