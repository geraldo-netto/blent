package com.blent.connection

import org.junit.Assert.*
import org.junit.Test
import java.nio.file.Files
import java.nio.file.Path
import java.nio.file.attribute.PosixFilePermissions

class AndroidPeerTrustStoreTest {
    private fun peer(value: Char) = ConnectionPeer.parse(value.toString().repeat(64))
    private fun record(revision: Long = 1, peers: List<ConnectionPeer> = listOf(peer('b'))) =
        ConnectionTrustCodec.encode(TrustSnapshot(peer('a'), revision, peers))
    private fun rejected(body: () -> Unit) { assertThrows(Exception::class.java, body) }
    private fun directory(): Path = Files.createTempDirectory("blent-t868-",
        PosixFilePermissions.asFileAttribute(PosixFilePermissions.fromString("rwx------")))
    private fun privateFile(path: Path, bytes: ByteArray) {
        Files.createFile(path, PosixFilePermissions.asFileAttribute(PosixFilePermissions.fromString("rw-------")))
        Files.write(path, bytes)
    }
    private fun cleanup(root: Path) { root.toFile().deleteRecursively() }
    private class Faults(private val delegate: TrustFileAccess) : TrustFileAccess by delegate {
        var selected = ""
        var sync = 0
        var failClose = false
        private fun checkpoint(name: String) { check(name != selected) { name } }
        override fun lock() { checkpoint("lock"); delegate.lock() }
        override fun writeNew(name: String, bytes: ByteArray) { checkpoint(name); delegate.writeNew(name, bytes) }
        override fun replace(source: String, target: String) { checkpoint("rename"); delegate.replace(source, target) }
        override fun syncDirectory() { checkpoint("sync${++sync}"); delegate.syncDirectory() }
        override fun remove(name: String) { checkpoint("remove"); delegate.remove(name) }
        override fun close() { delegate.close(); check(!failClose) }
    }

    @Test fun t868PrivateStoreRestartAndLifetimeOwnership() {
        val root = directory()
        try {
            val first = AndroidPeerTrustStore.open(root)
            assertNull(first.load())
            rejected { AndroidPeerTrustStore.open(root) }
            first.replace(record()); assertArrayEquals(record(), first.load())
            first.replace(record(2, emptyList()))
            first.close(); first.close()
            rejected { first.load() }; rejected { first.replace(record()) }
            AndroidPeerTrustStore.open(root).use { assertArrayEquals(record(2, emptyList()), it.load()) }
            assertTrue(Files.exists(root.resolve("trust.lock")))
            assertFalse(Files.exists(root.resolve("trust.pending")))
            assertFalse(Files.exists(root.resolve("trust.tmp")))
        } finally { cleanup(root) }
    }

    @Test fun t868EveryPublicationFailureIsExplicitAndCannotRestoreLiveAdmission() {
        for (point in listOf("trust.pending", "sync1", "trust.tmp", "rename", "sync2", "remove", "sync3")) {
            verifyFailure(point)
        }
        val root = directory()
        try {
            val files = Faults(NioTrustFileAccess.open(root)).apply { selected = "lock"; failClose = true }
            val failure = assertThrows(Exception::class.java) { AndroidPeerTrustStore.open(files) }
            assertEquals(1, failure.suppressed.size)
            AndroidPeerTrustStore.open(root).close()
        } finally { cleanup(root) }
    }

    private fun verifyFailure(point: String) {
        val root = directory()
        try {
            AndroidPeerTrustStore.open(root).use { it.replace(record()) }
            val files = Faults(NioTrustFileAccess.open(root))
            val store = AndroidPeerTrustStore.open(files)
            val pairing = ConnectionPairing()
            val ticket = pairing.begin(peer('b'), true, 0, 10)
            val lease = pairing.authenticated(ticket, peer('b'), 1)
            val trust = ConnectionTrust(store, peer('a'))
            files.selected = point
            rejected { trust.revoke(pairing, peer('b')) }
            assertFalse(trust.trusted(peer('b'))); assertFalse(pairing.current(lease))
            rejected { store.load() }; trust.close()
            verifyRestart(root, point)
        } finally { cleanup(root) }
    }

    private fun verifyRestart(root: Path, point: String) {
        AndroidPeerTrustStore.open(root).use { reopened ->
            when (point) {
                "trust.pending" -> assertArrayEquals(record(), reopened.load()) // No durable intent: failure explicitly reported.
                "sync3" -> assertArrayEquals(record(2, emptyList()), reopened.load()) // Replacement durable before marker removal.
                else -> rejected { reopened.load() }
            }
        }
    }

    @Test fun t868InterruptedAndUnsafeStateIsPreserved() {
        for (name in listOf("trust.pending", "trust.tmp")) {
            val root = directory()
            try {
                privateFile(root.resolve(name), byteArrayOf(1))
                AndroidPeerTrustStore.open(root).use { rejected { it.load() } }
                assertArrayEquals(byteArrayOf(1), Files.readAllBytes(root.resolve(name)))
            } finally { cleanup(root) }
        }
        for (name in listOf("trust.lock", "trust.bin")) {
            for (kind in listOf("symlink", "hardlink", "mode", "directory")) unsafe(rootName = name, kind = kind)
        }
    }

    private fun unsafe(rootName: String, kind: String) {
        val root = directory()
        try {
            val target = root.resolve("target"); privateFile(target, byteArrayOf(7))
            val child = root.resolve(rootName)
            when (kind) {
                "symlink" -> Files.createSymbolicLink(child, target)
                "hardlink" -> Files.createLink(child, target)
                "mode" -> { privateFile(child, byteArrayOf(1)); Files.setPosixFilePermissions(child, PosixFilePermissions.fromString("rw-r--r--")) }
                else -> Files.createDirectory(child)
            }
            rejected { AndroidPeerTrustStore.open(root).use { it.load() } }
            assertArrayEquals(byteArrayOf(7), Files.readAllBytes(target))
        } finally { cleanup(root) }
    }

    @Test fun t868NativeOperationBoundsAndCleanup() {
        val root = directory()
        try {
            val files = NioTrustFileAccess.open(root)
            files.lock(); rejected { files.lock() }
            rejected { files.exists("../escape") }
            rejected { files.read("trust.bin", -1) }
            rejected { files.read("trust.bin", ConnectionTrustCodec.MAX_RECORD + 1) }
            rejected { files.writeNew("trust.tmp", ByteArray(ConnectionTrustCodec.MAX_RECORD + 1)) }
            files.writeNew("trust.tmp", byteArrayOf())
            assertArrayEquals(byteArrayOf(), files.read("trust.tmp", 0))
            rejected { files.writeNew("trust.tmp", byteArrayOf(1)) }
            files.remove("trust.tmp")
            privateFile(root.resolve("trust.bin"), ByteArray(ConnectionTrustCodec.MAX_RECORD + 1))
            rejected { files.read("trust.bin", ConnectionTrustCodec.MAX_RECORD) }
            files.close(); files.close(); rejected { files.syncDirectory() }; rejected { files.exists("trust.bin") }
            Files.setPosixFilePermissions(root, PosixFilePermissions.fromString("rwxr-xr-x"))
            rejected { NioTrustFileAccess.open(root) }
        } finally { cleanup(root) }
        verifyInvalidPublication()
    }

    private fun verifyInvalidPublication() {
        val root = directory()
        try {
            AndroidPeerTrustStore.open(root).use {
                rejected { it.replace(byteArrayOf(1)) }; rejected { it.load() }
            }
            val files = Faults(NioTrustFileAccess.open(root)).apply { failClose = true }
            val store = AndroidPeerTrustStore.open(files)
            rejected { store.close() }; store.close()
        } finally { cleanup(root) }
    }
}
