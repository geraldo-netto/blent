package com.blent.connection

import java.nio.ByteBuffer
import java.nio.channels.FileChannel
import java.nio.channels.FileLock
import java.nio.file.Files
import java.nio.file.LinkOption.NOFOLLOW_LINKS
import java.nio.file.NoSuchFileException
import java.nio.file.Path
import java.nio.file.StandardCopyOption.ATOMIC_MOVE
import java.nio.file.StandardCopyOption.REPLACE_EXISTING
import java.nio.file.StandardOpenOption.*
import java.nio.file.attribute.PosixFileAttributes
import java.nio.file.attribute.PosixFilePermissions
import java.nio.file.attribute.UserPrincipal

/** Stock API26+ filesystem provider. Caller supplies the application's private,
 * stable directory; this is not a sandbox against other code with the same UID.
 * A device must qualify atomic rename and directory force before runtime use. */
internal class NioTrustFileAccess private constructor(
    private val directory: Path,
    private val owner: UserPrincipal,
    private val directoryChannel: FileChannel,
) : TrustFileAccess {
    private var lockChannel: FileChannel? = null
    private var heldLock: FileLock? = null
    private var closed = false

    companion object {
        fun open(directory: Path): NioTrustFileAccess {
            val attributes = Files.readAttributes(directory, PosixFileAttributes::class.java, NOFOLLOW_LINKS)
            require(attributes.isDirectory && attributes.permissions() == PosixFilePermissions.fromString("rwx------"))
            val channel = FileChannel.open(directory, READ, NOFOLLOW_LINKS)
            return NioTrustFileAccess(directory, attributes.owner(), channel)
        }
    }

    private fun path(name: String): Path {
        check(!closed)
        require(name in setOf("trust.lock", "trust.pending", "trust.tmp", "trust.bin"))
        return directory.resolve(name)
    }

    private fun validate(path: Path) {
        val attributes = Files.readAttributes(path, PosixFileAttributes::class.java, NOFOLLOW_LINKS)
        require(attributes.isRegularFile && attributes.owner() == owner)
        require(attributes.permissions() == PosixFilePermissions.fromString("rw-------"))
        require((Files.getAttribute(path, "unix:nlink", NOFOLLOW_LINKS) as Number).toLong() == 1L)
    }

    override fun lock() {
        check(heldLock == null && lockChannel == null)
        val selected = path("trust.lock")
        if (exists("trust.lock")) validate(selected)
        val channel = FileChannel.open(selected, setOf(READ, WRITE, CREATE, NOFOLLOW_LINKS),
            PosixFilePermissions.asFileAttribute(PosixFilePermissions.fromString("rw-------")))
        lockChannel = channel
        validate(selected)
        heldLock = checkNotNull(channel.tryLock()) { "Trust storage owned by another worker" }
    }

    override fun exists(name: String): Boolean = try {
        Files.readAttributes(path(name), PosixFileAttributes::class.java, NOFOLLOW_LINKS)
        true
    } catch (_: NoSuchFileException) { false }

    override fun read(name: String, maximum: Int): ByteArray? {
        require(maximum in 0..ConnectionTrustCodec.MAX_RECORD)
        if (!exists(name)) return null
        val selected = path(name)
        validate(selected)
        return FileChannel.open(selected, READ, NOFOLLOW_LINKS).use { channel ->
            val bytes = ByteBuffer.allocate(maximum + 1)
            while (bytes.hasRemaining() && channel.read(bytes) != -1) { /* bounded regular file */ }
            require(bytes.position() <= maximum)
            bytes.array().copyOf(bytes.position())
        }
    }

    override fun writeNew(name: String, bytes: ByteArray) {
        require(bytes.size <= ConnectionTrustCodec.MAX_RECORD)
        FileChannel.open(path(name), setOf(WRITE, CREATE_NEW, NOFOLLOW_LINKS),
            PosixFilePermissions.asFileAttribute(PosixFilePermissions.fromString("rw-------"))).use { channel ->
            val buffer = ByteBuffer.wrap(bytes)
            while (buffer.hasRemaining()) channel.write(buffer)
            channel.force(true)
        }
    }

    override fun replace(source: String, target: String) {
        validate(path(source))
        if (exists(target)) validate(path(target))
        Files.move(path(source), path(target), ATOMIC_MOVE, REPLACE_EXISTING)
    }

    override fun syncDirectory() { check(!closed); directoryChannel.force(true) }
    override fun remove(name: String) { Files.delete(path(name)) }

    override fun close() {
        if (closed) return
        closed = true
        try { lockChannel?.close() } finally { directoryChannel.close() }
    }
}
