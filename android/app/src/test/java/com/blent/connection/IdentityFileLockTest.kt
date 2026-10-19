package com.blent.connection

import java.io.File
import java.io.IOException
import java.io.RandomAccessFile
import java.nio.channels.ClosedChannelException
import java.nio.channels.OverlappingFileLockException
import java.nio.file.Files
import java.nio.file.Path
import java.nio.file.attribute.PosixFilePermissions
import org.junit.Assert.*
import org.junit.Test

class IdentityFileLockTest {
    private fun directory() = Files.createTempDirectory("blent-t1157-",
        PosixFilePermissions.asFileAttribute(PosixFilePermissions.fromString("rwx------")))
    private class FaultFile(path: File) : RandomAccessFile(path, "rw") {
        var failClose = false
        override fun close() { if (failClose) throw IOException("T1157 original close is unknown"); super.close() }
    }

    @Test fun t1157AliasesRejectBeforeNativeOpenAndUnrelatedDirectoriesRemainAvailable() {
        val root = directory(); val other = directory()
        try {
            IdentityFileLock.open(root.toFile()).use {
                val alias = root.resolve("..").resolve(root.fileName)
                assertThrows(OverlappingFileLockException::class.java) {
                    IdentityFileLock.open(alias.toFile(), openFile = { error("T1157 duplicate reached native open") })
                }
                IdentityFileLock.open(other.toFile()).close()
            }
            val next = IdentityFileLock.open(root.toFile()); next.close(); next.close()
        } finally { root.toFile().deleteRecursively(); other.toFile().deleteRecursively() }
    }

    @Test fun t1157FailedAcquisitionAndInvalidDirectoryReturnNoNativeAuthority() {
        val root = directory()
        val owners = TrustDirectoryOwnership(1)
        try {
            assertThrows(IOException::class.java) {
                IdentityFileLock.open(root.toFile(), owners) { throw IOException("T1157 before native acquisition") }
            }
            val file = RandomAccessFile(root.resolve("accessory-identity.lock").toFile(), "rw").apply { close() }
            assertThrows(ClosedChannelException::class.java) { IdentityFileLock.open(root.toFile(), owners) { file } }
            IdentityFileLock.open(root.toFile(), owners).close()
            val regular = Files.createFile(root.resolve("regular"))
            assertThrows(IllegalArgumentException::class.java) { IdentityFileLock.open(regular.toFile(), owners) }
            val link = Files.createSymbolicLink(root.resolve("link"), root)
            assertThrows(IllegalArgumentException::class.java) { IdentityFileLock.open(link.toFile(), owners) }
            assertThrows(IOException::class.java) { IdentityFileLock.open(root.resolve("missing").toFile(), owners) }
        } finally { root.toFile().deleteRecursively() }
    }

    private fun retained(root: Path, owners: TrustDirectoryOwnership) {
        assertThrows(OverlappingFileLockException::class.java) {
            IdentityFileLock.open(root.toFile(), owners) { error("T1157 unknown close was reused") }
        }
        assertThrows(IllegalStateException::class.java) { owners.reserve(Any()) }
    }

    @Test fun t1157NativeCloseFailureRetainsItsBoundedSlotWithoutRetryProof() {
        val root = directory()
        val owners = TrustDirectoryOwnership(1)
        val file = FaultFile(root.resolve("accessory-identity.lock").toFile())
        try {
            val owned = IdentityFileLock.open(root.toFile(), owners) { file }
            file.failClose = true
            assertThrows(IOException::class.java) { owned.close() }
            owned.close() // Logical idempotence cannot release the unknown obligation.
            retained(root, owners)
        } finally {
            file.failClose = false; file.close() // Independent test sponsor actually settles the descriptor.
            root.toFile().deleteRecursively()
        }
    }

    @Test fun t1157PartialAcquisitionCloseFailurePreservesOriginalErrorAndReservation() {
        val root = directory()
        val owners = TrustDirectoryOwnership(1)
        val file = FaultFile(root.resolve("accessory-identity.lock").toFile()).apply { close(); failClose = true }
        try {
            val failure = assertThrows(ClosedChannelException::class.java) {
                IdentityFileLock.open(root.toFile(), owners) { file }
            }
            assertEquals(1, failure.suppressed.size)
            assertTrue(failure.suppressed.single() is IOException)
            retained(root, owners)
        } finally { file.failClose = false; file.close(); root.toFile().deleteRecursively() }
    }

    @Test fun t1157FailedLockReleaseStillClosesOriginalFileBeforeReturningAdmission() {
        val root = directory()
        val owners = TrustDirectoryOwnership(1)
        val file = RandomAccessFile(root.resolve("accessory-identity.lock").toFile(), "rw")
        try {
            val owned = IdentityFileLock.open(root.toFile(), owners) { file }
            file.channel.close() // Deliberate already-settled native-handle fault, never an invented dangling pointer.
            assertThrows(ClosedChannelException::class.java) { owned.close() }
            IdentityFileLock.open(root.toFile(), owners).close()
        } finally { file.close(); root.toFile().deleteRecursively() }
    }
}
