package com.blent.connection

import java.nio.file.Files
import java.nio.file.Path
import java.nio.file.attribute.PosixFilePermissions
import java.util.concurrent.TimeUnit
import org.junit.Assert.*
import org.junit.Test

class TrustKernelLockTest {
    private data class Identity(val device: String, val inode: String)
    private data class Observed(val before: Int, val after: Int, val released: Int, val rejected: Boolean)
    private fun identity(root: Path): Identity {
        val values = Files.readAttributes(root.resolve("trust.lock"), "unix:dev,ino")
        return Identity(values["dev"].toString(), values["ino"].toString())
    }
    private fun probe(root: Path, original: Identity): Int {
        val program = root.resolve("probe.py")
        javaClass.classLoader!!.getResourceAsStream("trust-lock-probe.py")!!.use { source ->
            Files.newOutputStream(program).use { source.copyTo(it) }
        }
        val file = root.resolve("trust.lock")
        val process = ProcessBuilder("/usr/bin/python3", program.toString(), file.toString(), original.device, original.inode).start()
        return try {
            check(process.waitFor(5, TimeUnit.SECONDS)) { "T1156 independent probe protocol did not return" }
            process.exitValue()
        } finally {
            if (process.isAlive) process.destroyForcibly()
            process.waitFor() // Actual original child exit/reap, never just the request.
            process.inputStream.close(); process.errorStream.close(); process.outputStream.close()
        }
    }
    @Test fun t1156RejectedSameJvmOpenCannotReleaseOriginalKernelLock() {
        val root = Files.createTempDirectory("blent-t1156-", PosixFilePermissions.asFileAttribute(PosixFilePermissions.fromString("rwx------")))
        val observed = try { observe(root) } finally {
            Files.walk(root).use { paths -> paths.sorted(Comparator.reverseOrder()).forEach(Files::delete) }
        }
        assertTrue("T1156 second JVM owner was admitted", observed.rejected)
        assertEquals("T1156 original kernel lock was never held", 1, observed.before)
        assertEquals("T1156 rejected open released original kernel lock", 1, observed.after)
        assertEquals("T1156 original close did not settle the kernel lock", 0, observed.released)
    }
    private fun observe(root: Path): Observed {
        val first = AndroidPeerTrustStore.open(root)
        val original = identity(root)
        val before: Int
        val after: Int
        val rejected: Boolean
        try {
            before = probe(root, original)
            val attempt = runCatching { AndroidPeerTrustStore.open(root) }
            rejected = attempt.isFailure
            attempt.getOrNull()?.close()
            after = probe(root, original)
        } finally { first.close() }
        return Observed(before, after, probe(root, original), rejected)
    }
}
