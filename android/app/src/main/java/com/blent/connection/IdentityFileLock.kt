package com.blent.connection

import java.io.File
import java.io.RandomAccessFile
import java.nio.channels.FileLock
import java.nio.file.Files
import java.nio.file.LinkOption.NOFOLLOW_LINKS
import java.nio.file.NoSuchFileException
import java.nio.file.Path
import java.nio.file.attribute.PosixFileAttributes
import java.nio.file.attribute.UserPrincipal

/** One original identity-lock file in a stable application-private directory.
 * All in-process opens of that lock must use this adapter. Unknown native close
 * keeps its bounded slot until original process exit; logical close is no proof.
 * The injected opener returns the original file for the selected path and owns
 * failed acquisition without abandoning an unreturned descriptor. Production
 * uses the stock RandomAccessFile constructor. This is not a same-UID sandbox. */
internal class IdentityFileLock private constructor(
    private val file: RandomAccessFile,
    private val lock: FileLock,
    private val ownership: TrustDirectoryOwnership.Lease,
) : AutoCloseable {
    private var closed = false

    companion object {
        private val directories = TrustDirectoryOwnership()
        fun open(
            directory: File,
            owners: TrustDirectoryOwnership = directories,
            openFile: (File) -> RandomAccessFile = { RandomAccessFile(it, "rw") },
        ): IdentityFileLock {
            val root = directory.toPath()
            val attributes = Files.readAttributes(root, PosixFileAttributes::class.java, NOFOLLOW_LINKS)
            require(attributes.isDirectory)
            val ownership = owners.reserve(checkNotNull(attributes.fileKey()) { "Stable identity directory unavailable" })
            val selected = root.resolve("accessory-identity.lock")
            val file = try { validateChild(selected, attributes.owner()); openFile(selected.toFile()) }
            catch (failure: Exception) { ownership.close(); throw failure }
            return try {
                validateChild(selected, attributes.owner())
                val lock = checkNotNull(file.channel.tryLock()) { "Client identity is owned by another worker" }
                IdentityFileLock(file, lock, ownership)
            } catch (failure: Exception) { abandon(file, ownership, failure) }
        }

        /** Validation only, never ownership proof. The opener holds the original
         * directory slot and stable private namespace across validation/open. */
        internal fun validateChild(path: Path, owner: UserPrincipal) {
            val attributes = try { Files.readAttributes(path, PosixFileAttributes::class.java, NOFOLLOW_LINKS) }
            catch (_: NoSuchFileException) { return }
            require(attributes.isRegularFile && attributes.owner() == owner)
            require((Files.getAttribute(path, "unix:nlink", NOFOLLOW_LINKS) as Number).toLong() == 1L)
        }

        private fun abandon(file: RandomAccessFile, ownership: TrustDirectoryOwnership.Lease, failure: Exception): Nothing {
            try { file.close() }
            catch (cleanup: Exception) { failure.addSuppressed(cleanup); throw failure }
            ownership.close()
            throw failure
        }
    }

    @Synchronized override fun close() {
        if (closed) return
        closed = true
        try { lock.release() }
        finally {
            file.close() // Actual original file/channel close also fences a failed lock release.
            ownership.close()
        }
    }
}
