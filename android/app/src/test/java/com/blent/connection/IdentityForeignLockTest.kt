package com.blent.connection

import java.io.IOException
import java.nio.file.Files
import java.nio.file.Path
import java.nio.file.attribute.PosixFilePermissions
import java.util.concurrent.TimeUnit
import org.junit.Assert.*
import org.junit.Test

class IdentityForeignLockTest {
    private fun start(root: Path): Process {
        val program = root.resolve("owner.py")
        javaClass.classLoader!!.getResourceAsStream("identity-lock-owner.py")!!.use { source ->
            Files.newOutputStream(program).use { source.copyTo(it) }
        }
        val file = Files.createFile(root.resolve("accessory-identity.lock"),
            PosixFilePermissions.asFileAttribute(PosixFilePermissions.fromString("rw-------")))
        val identity = Files.readAttributes(file, "unix:dev,ino")
        return ProcessBuilder("/usr/bin/python3", program.toString(), file.toString(), root.resolve("ready").toString(),
            identity["dev"].toString(), identity["ino"].toString()).start()
    }
    private fun awaitReady(root: Path, process: Process) {
        val end = System.nanoTime() + TimeUnit.SECONDS.toNanos(5)
        while (!Files.exists(root.resolve("ready")) && process.isAlive && System.nanoTime() < end) Thread.sleep(1)
        check(process.isAlive && Files.exists(root.resolve("ready"))) { "T1157 foreign owner protocol failed" }
        assertArrayEquals(byteArrayOf('H'.code.toByte()), Files.readAllBytes(root.resolve("ready")))
    }
    private fun settle(process: Process) {
        try {
            try { process.outputStream.write('R'.code); process.outputStream.flush() } catch (_: IOException) { /* Already exited. */ }
            if (!process.waitFor(5, TimeUnit.SECONDS)) process.destroyForcibly()
            process.waitFor() // Request/marker/deadline never substitutes for actual original exit.
        } finally { closeConnectionResources(listOf(process.inputStream, process.errorStream, process.outputStream)) }
    }

    @Test fun t1157ForeignProcessLockRejectsWithoutLeakingLocalAdmission() {
        val root = Files.createTempDirectory("blent-t1157-foreign-",
            PosixFilePermissions.asFileAttribute(PosixFilePermissions.fromString("rwx------")))
        try {
            val process = start(root)
            val owners = TrustDirectoryOwnership(1)
            val failure = try {
                awaitReady(root, process)
                runCatching { IdentityFileLock.open(root.toFile(), owners).use {} }.exceptionOrNull()
            } finally { settle(process) }
            assertEquals(0, process.exitValue())
            assertEquals(IllegalStateException::class.java, failure?.javaClass)
            IdentityFileLock.open(root.toFile(), owners).close()
        } finally { root.toFile().deleteRecursively() }
    }
}
