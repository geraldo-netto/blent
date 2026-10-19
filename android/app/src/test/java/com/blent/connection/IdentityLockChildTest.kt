package com.blent.connection

import java.nio.file.Files
import java.nio.file.attribute.PosixFilePermissions
import java.nio.file.attribute.UserPrincipal
import org.junit.Assert.*
import org.junit.Test

class IdentityLockChildTest {
    @Test fun t1157UnsafeChildStateRejectsBeforeNativeDescriptorOpening() {
        for (kind in listOf("symlink", "hardlink", "directory")) checkChild(kind)
    }
    private fun checkChild(kind: String) {
        val root = Files.createTempDirectory("blent-t1157-child-",
            PosixFilePermissions.asFileAttribute(PosixFilePermissions.fromString("rwx------")))
        val owners = TrustDirectoryOwnership(1)
        try {
            val target = Files.write(root.resolve("target"), byteArrayOf(7))
            val child = root.resolve("accessory-identity.lock")
            when (kind) {
                "symlink" -> Files.createSymbolicLink(child, target)
                "hardlink" -> Files.createLink(child, target)
                else -> Files.createDirectory(child)
            }
            assertThrows(IllegalArgumentException::class.java) {
                IdentityFileLock.open(root.toFile(), owners) { error("T1157 unsafe child reached native open") }
            }
            assertArrayEquals(byteArrayOf(7), Files.readAllBytes(target))
            Files.delete(child)
            IdentityFileLock.open(root.toFile(), owners).close()
        } finally { root.toFile().deleteRecursively() }
    }
    @Test fun t1157ForeignOwnerMetadataFailsWithoutNativeOpenOrMutation() {
        val root = Files.createTempDirectory("blent-t1157-owner-",
            PosixFilePermissions.asFileAttribute(PosixFilePermissions.fromString("rwx------")))
        try {
            val child = Files.write(root.resolve("accessory-identity.lock"), byteArrayOf(7))
            val foreign = UserPrincipal { "T1157 unrelated immutable owner metadata" }
            assertThrows(IllegalArgumentException::class.java) { IdentityFileLock.validateChild(child, foreign) }
            assertArrayEquals(byteArrayOf(7), Files.readAllBytes(child))
        } finally { root.toFile().deleteRecursively() }
    }
}
