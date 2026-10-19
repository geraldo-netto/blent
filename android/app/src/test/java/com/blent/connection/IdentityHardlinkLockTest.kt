package com.blent.connection

import java.io.RandomAccessFile
import java.nio.file.Files
import java.nio.file.Path
import java.nio.file.attribute.PosixFilePermissions
import org.junit.Assert.*
import org.junit.Test

class IdentityHardlinkLockTest {
    private data class Observed(val before: Int, val after: Int, val released: Int, val opened: Boolean, val failure: Throwable?)
    private fun directory() = Files.createTempDirectory("blent-t1157-alias-",
        PosixFilePermissions.asFileAttribute(PosixFilePermissions.fromString("rwx------")))

    @Test fun t1157HardlinkedIdentityLockRejectsBeforeOpeningAnotherOriginalDescriptor() {
        val root = directory(); val other = directory()
        val observed = try { observe(root, other) }
        finally { root.toFile().deleteRecursively(); other.toFile().deleteRecursively() }
        assertEquals(1, observed.before)
        assertEquals("T1157 hardlink rejection released original kernel lock", 1, observed.after)
        assertEquals(0, observed.released)
        assertFalse("T1157 hardlink reached native descriptor open", observed.opened)
        assertTrue(observed.failure is IllegalArgumentException)
    }
    private fun observe(root: Path, other: Path): Observed {
        val first = IdentityFileLock.open(root.toFile())
        val file = root.resolve("accessory-identity.lock")
        val original = NativeFileLockProbe.identity(file)
        val result = try {
            Files.createLink(other.resolve("accessory-identity.lock"), file)
            val before = NativeFileLockProbe.held(file, original)
            var opened = false
            val failure = runCatching {
                IdentityFileLock.open(other.toFile(), openFile = { opened = true; RandomAccessFile(it, "rw") }).use {}
            }.exceptionOrNull()
            Observed(before, NativeFileLockProbe.held(file, original), -1, opened, failure)
        } finally { first.close() }
        return result.copy(released = NativeFileLockProbe.held(file, original))
    }
}
