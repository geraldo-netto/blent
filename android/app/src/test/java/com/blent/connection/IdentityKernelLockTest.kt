package com.blent.connection

import java.io.IOException
import java.nio.channels.OverlappingFileLockException
import java.nio.file.Files
import java.nio.file.Path
import java.nio.file.attribute.PosixFilePermissions
import java.security.KeyStore
import java.util.concurrent.CountDownLatch
import java.util.concurrent.FutureTask
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicInteger
import org.junit.Assert.*
import org.junit.Test

class IdentityKernelLockTest {
    private data class Identity(val device: String, val inode: String)
    private data class Held(val identity: Identity, val before: Int, val after: Int, val rejected: Boolean)
    private data class Observed(val held: Held, val released: Int, val secondaryReads: Int, val originalFailed: Boolean)

    private fun identity(root: Path): Identity {
        val values = Files.readAttributes(root.resolve("accessory-identity.lock"), "unix:dev,ino")
        return Identity(values["dev"].toString(), values["ino"].toString())
    }
    private fun probe(root: Path, original: Identity): Int {
        val program = root.resolve("probe.py")
        javaClass.classLoader!!.getResourceAsStream("trust-lock-probe.py")!!.use { source ->
            Files.newOutputStream(program).use { source.copyTo(it) }
        }
        val process = ProcessBuilder("/usr/bin/python3", program.toString(),
            root.resolve("accessory-identity.lock").toString(), original.device, original.inode).start()
        return try {
            check(process.waitFor(5, TimeUnit.SECONDS)) { "T1157 independent probe protocol did not return" }
            process.exitValue()
        } finally { settle(process) }
    }
    private fun settle(process: Process) {
        try {
            if (process.isAlive) process.destroyForcibly()
            process.waitFor() // Actual original child exit/reap.
        } finally { closeConnectionResources(listOf(process.inputStream, process.errorStream, process.outputStream)) }
    }

    @Test fun t1157RejectedSameJvmIdentityLoadPreservesOriginalKernelLock() {
        val root = Files.createTempDirectory("blent-t1157-",
            PosixFilePermissions.asFileAttribute(PosixFilePermissions.fromString("rwx------")))
        val observed = try { observe(root) } finally {
            Files.walk(root).use { paths -> paths.sorted(Comparator.reverseOrder()).forEach(Files::delete) }
        }
        assertTrue("T1157 original provider operation did not settle", observed.originalFailed)
        assertEquals("T1157 rejected duplicate touched provider", 0, observed.secondaryReads)
        assertTrue("T1157 duplicate was not rejected by original ownership", observed.held.rejected)
        assertEquals("T1157 original native lock was never held", 1, observed.held.before)
        assertEquals("T1157 rejected identity load released original kernel lock", 1, observed.held.after)
        assertEquals("T1157 original actual close failed to release lock", 0, observed.released)
    }

    private fun blockedStore(entered: CountDownLatch, release: CountDownLatch) = object : IdentityStore {
        override fun contains(alias: String): Boolean {
            entered.countDown(); release.await()
            throw IOException("T1157 synthetic original provider returned")
        }
        override fun read(alias: String): KeyStore.PrivateKeyEntry = error("T1157 unexpected private entry read")
    }
    private fun observe(root: Path): Observed {
        val entered = CountDownLatch(1); val release = CountDownLatch(1)
        val store = blockedStore(entered, release)
        val task = FutureTask { runCatching {
            AndroidConnectionIdentity(root.toFile(), store, IdentityGenerator { error("T1157 unexpected generation") }).load()
        } }
        val original = Thread(task, "blent-t1157-original")
        val reads = AtomicInteger()
        original.start()
        val held = try {
            check(entered.await(5, TimeUnit.SECONDS)) { "T1157 original provider was not reached" }
            competing(root, reads)
        } finally { release.countDown(); original.join() }
        return Observed(held, probe(root, held.identity), reads.get(), task.get().exceptionOrNull() is IOException)
    }
    private fun competing(root: Path, reads: AtomicInteger): Held {
        val original = identity(root)
        val before = probe(root, original)
        val store = object : IdentityStore {
            override fun contains(alias: String): Boolean { reads.incrementAndGet(); error("T1157 duplicate reached provider") }
            override fun read(alias: String): KeyStore.PrivateKeyEntry = error("T1157 unexpected private entry read")
        }
        val attempted = runCatching {
            AndroidConnectionIdentity(root.toFile(), store, IdentityGenerator { error("T1157 unexpected generation") }).load()
        }
        return Held(original, before, probe(root, original), attempted.exceptionOrNull() is OverlappingFileLockException)
    }
}
