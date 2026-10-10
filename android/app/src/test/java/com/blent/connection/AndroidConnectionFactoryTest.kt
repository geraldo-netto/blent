package com.blent.connection

import android.app.Application
import android.content.Context
import android.content.ContextWrapper
import android.hardware.usb.UsbAccessory
import android.hardware.usb.UsbManager
import java.io.File
import java.nio.file.Files
import java.nio.file.Path
import java.nio.file.attribute.PosixFilePermissions
import java.security.GeneralSecurityException
import kotlinx.coroutines.*
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.RuntimeEnvironment
import org.robolectric.Shadows.shadowOf
import org.robolectric.annotation.Config

private class FactoryContext(app: Application, private val directory: File) : ContextWrapper(app) {
    override fun getNoBackupFilesDir(): File = directory
}

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [27, 34])
class AndroidConnectionFactoryTest {
    @get:Rule val temporary = TemporaryFolder()
    private val app: Application get() = RuntimeEnvironment.getApplication()
    private val manager: UsbManager get() = app.getSystemService(UsbManager::class.java)
    private fun context(): Context = FactoryContext(app, temporary.root)
    private fun device(): UsbAccessory = UsbAccessory::class.java.getDeclaredConstructor(*Array(6) { String::class.java })
        .newInstance("Blent", "Blent", "Blent connection", "1", "", "fixture")
    private fun selected(device: UsbAccessory): AccessorySelection = AccessorySelectionOwner(AndroidConnectionFactory.profile)
        .select(listOf(AndroidAccessoryInventory.describe(device)))
    private fun spec(device: UsbAccessory) = ConnectionWorkerSpec(selected(device).encode(),
        session = ConnectionSessionLimits(capacity = 4096), io = ConnectionRunLimits(100, 100), nativeTimeoutMs = 500, timerMs = 1)
    private fun factory(): AndroidConnectionFactory = AndroidConnectionFactory(::context)
    @Test fun t854FactoryCreateAndUnexecutedRetirementAreNativeInert() = runBlocking {
        var queries = 0
        val factory = AndroidConnectionFactory({ queries++; context() })
        val worker = factory.create(spec(device())) { error("No native work means no event") }
        assertEquals(0, queries); assertEquals(emptyList<File>(), temporary.root.listFiles()!!.toList())
        worker.retire(); assertEquals(0, queries)
    }
    @Test fun t854MalformedOpaqueSelectionCannotReachNativeFactory() {
        var queries = 0; val factory = AndroidConnectionFactory({ queries++; context() })
        val values = listOf("", "0:" + "ab".repeat(32), "../device", "1:" + "AB".repeat(32), "01:" + "ab".repeat(32), "9223372036854775808:" + "ab".repeat(32))
        for (value in values) assertThrows(IllegalArgumentException::class.java) { factory.create(ConnectionWorkerSpec(value)) {} }
        assertEquals(0, queries)
    }
    @Test fun t854PrivateDirectoryCreationAndReusePreserveState() {
        val factory = factory(); val directory = factory.directory()
        assertEquals(temporary.root.toPath().resolve("blent-connection"), directory)
        assertEquals(PosixFilePermissions.fromString("rwx------"), Files.getPosixFilePermissions(directory))
        val marker = directory.resolve("existing"); Files.write(marker, byteArrayOf(1, 2))
        assertEquals(directory, factory.directory()); assertArrayEquals(byteArrayOf(1, 2), Files.readAllBytes(marker))
    }
    @Test fun t854FileSymlinkAndInsecureDirectoryRejectWithoutRepair() {
        val factory = factory(); val path = temporary.root.toPath().resolve("blent-connection")
        Files.write(path, byteArrayOf(9))
        assertThrows(IllegalArgumentException::class.java) { factory.directory() }
        assertArrayEquals(byteArrayOf(9), Files.readAllBytes(path)); Files.delete(path)
        val target = temporary.newFolder("target").toPath(); Files.createSymbolicLink(path, target)
        assertThrows(IllegalArgumentException::class.java) { factory.directory() }
        assertEquals(target, Files.readSymbolicLink(path)); Files.delete(path)
        Files.createDirectory(path); val exposed = PosixFilePermissions.fromString("rwxr-x---")
        Files.setPosixFilePermissions(path, exposed)
        assertThrows(IllegalArgumentException::class.java) { factory.directory() }
        assertEquals(exposed, Files.getPosixFilePermissions(path))
    }
    @Test fun t854NativeOpeningRequiresProductionProfileCurrentSelectionAndGrant() {
        val factory = factory(); val device = device(); val spec = spec(device)
        shadowOf(manager).setAttachedUsbAccessory(device)
        val denied = factory.opening(spec, selected(device), { true })
        assertThrows(IllegalStateException::class.java) { denied.open() }; denied.close(); assertTrue(denied.retired())
        shadowOf(manager).grantPermission(device)
        val cancelled = factory.opening(spec, selected(device), { false })
        assertThrows(IllegalStateException::class.java) { cancelled.open() }; cancelled.close(); assertTrue(cancelled.retired())
        val opening = factory.opening(spec, selected(device), { true })
        opening.open(); opening.close(); accessoryAwait(opening::retired)
        shadowOf(manager).setAttachedUsbAccessory(nativeAccessory())
        val wrong = factory.opening(spec, selected(device), { true })
        assertThrows(IllegalStateException::class.java) { wrong.open() }; wrong.close(); assertTrue(wrong.retired())
    }
    @Test fun t854MissingUsbServiceRejectsWithoutCreatingNativeOpening() {
        val context = object : ContextWrapper(context()) {
            override fun getSystemService(name: String): Any? = null
        }
        val device = device(); val factory = AndroidConnectionFactory({ context })
        assertThrows(IllegalStateException::class.java) { factory.opening(spec(device), selected(device), { true }) }
    }
    private suspend fun fails(worker: ConnectionWorker): Exception {
        try {
            withTimeout(3000) { worker.execute() }
            throw AssertionError("Unconnected fixture accessory cannot complete authenticated setup")
        } catch (failure: Exception) { return failure }
        finally { worker.cancel(); withTimeout(3000) { worker.retire() } }
    }
    @Test fun t854ProductionWorkerLoadsIdentityAndTrustOnlyInsideOwnedSetup() = runBlocking {
        val device = device(); shadowOf(manager).setAttachedUsbAccessory(device); shadowOf(manager).grantPermission(device)
        val directories = java.util.Collections.synchronizedList(ArrayList<Path>()); val store = SetupStore()
        val factory = AndroidConnectionFactory(::context, { System.nanoTime() / 1000000 },
            { path -> directories.add(path); SocketSetupFixture.identity() },
            { path -> directories.add(path); store })
        val worker = factory.create(spec(device)) {}
        assertTrue(directories.isEmpty())
        val failure = fails(worker)
        assertFalse(failure is TimeoutCancellationException)
        assertEquals(listOf(factory.directory(), factory.directory()), directories.toList()); assertTrue(store.closed)
    }
    @Test fun t854ProductionDefaultTrustAdapterUsesPrivatePersistentStorage() = runBlocking {
        val device = device(); shadowOf(manager).setAttachedUsbAccessory(device); shadowOf(manager).grantPermission(device)
        val factory = AndroidConnectionFactory(::context, { System.nanoTime() / 1000000 }, { SocketSetupFixture.identity() })
        val failure = fails(factory.create(spec(device)) {})
        assertFalse(failure is TimeoutCancellationException)
        assertTrue(Files.isRegularFile(factory.directory().resolve("trust.lock")))
        AndroidPeerTrustStore.open(factory.directory()).use { assertNull(it.load()) }
    }
    @Test fun t854DefaultLoaderAndFactoryUseExistingIdentityProviderWithoutRotation() = runBlocking {
        val provider = object : java.security.Provider("BlentFactoryFixture", 1.0, "Synthetic test identity only") {}
        provider.put("KeyStore.AndroidKeyStore", ConnectionFactoryKeyStoreFixture::class.java.name)
        check(java.security.Security.insertProviderAt(provider, 1) == 1)
        try {
            val factory = factory()
            val identity = loadAndroidConnectionIdentity(factory.directory())
            assertArrayEquals(SocketSetupFixture.bytes("tablet.der"), identity.publicIdentity().certificate())
            val device = device(); shadowOf(manager).setAttachedUsbAccessory(device); shadowOf(manager).grantPermission(device)
            val failure = fails(factory.create(spec(device)) {})
            assertFalse(failure is TimeoutCancellationException)
            assertTrue(Files.isRegularFile(factory.directory().resolve("trust.lock")))
        } finally { java.security.Security.removeProvider(provider.name) }
    }
    @Test fun t854UnavailableJvmAndroidKeyStoreIsReportedWithoutInventedIdentity() {
        val directory = factory().directory()
        val failure = assertThrows(GeneralSecurityException::class.java) { loadAndroidConnectionIdentity(directory) }
        assertTrue(failure.message.orEmpty().contains("AndroidKeyStore"))
        assertFalse(Files.exists(directory.resolve("trust.bin")))
    }
}
