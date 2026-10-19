package com.blent.connection

import java.io.IOException
import java.nio.channels.OverlappingFileLockException
import java.nio.file.Files
import java.nio.file.Path
import java.nio.file.attribute.PosixFilePermissions
import org.junit.Assert.*
import org.junit.Test

class TrustDirectoryNativeTest {
    private fun directory(): Path = Files.createTempDirectory("blent-t1156-",
        PosixFilePermissions.asFileAttribute(PosixFilePermissions.fromString("rwx------")))

    @Test fun t1156AliasRejectsBeforeOpeningAndIndependentDirectoriesRemainAvailable() {
        val root = directory()
        val other = directory()
        try {
            NioTrustFileAccess.open(root).use { first ->
                first.lock()
                val alias = root.resolve("..").resolve(root.fileName)
                assertThrows(OverlappingFileLockException::class.java) {
                    NioTrustFileAccess.open(alias) { error("T1156 duplicate reached native open") }
                }
                NioTrustFileAccess.open(other).use { it.lock() }
            }
            AndroidPeerTrustStore.open(root).close()
        } finally { root.toFile().deleteRecursively(); other.toFile().deleteRecursively() }
    }

    @Test fun t1156RejectedNativeAcquisitionReturnsOnlyItsOriginalReservation() {
        val root = directory()
        try {
            assertThrows(IOException::class.java) {
                NioTrustFileAccess.open(root) { throw IOException("T1156 failure before native acquisition") }
            }
            AndroidPeerTrustStore.open(root).close()
            val original = NioTrustFileAccess.open(root)
            val files = object : TrustFileAccess by original {
                override fun close() { throw IOException("T1156 unknown original close") }
            }
            try {
                AndroidPeerTrustStore.open(files).use { assertNull(it.load()) }
                fail("T1156 unknown close reported success")
            } catch (_: IOException) {
                assertThrows(OverlappingFileLockException::class.java) { AndroidPeerTrustStore.open(root) }
            } finally { original.close() } // Test sponsor actually settles the retained original.
            AndroidPeerTrustStore.open(root).close()
        } finally { root.toFile().deleteRecursively() }
    }
}
