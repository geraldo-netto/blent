package com.blent.connection

/** Independent local media metadata/choice and completion command boundaries.
 * Contracts contain no implementation, OS device or worker handle. */
internal interface LocalMediaOffer<P : ConnectionManagement.Profile> {
    val lease: ConnectionResourceLease
    val profile: P
}

internal interface LocalMediaRoute<P : ConnectionManagement.Profile> {
    val lease: ConnectionResourceLease
    val credential: ConnectionManagement.Credential
    val port: Int
    val profile: P
}

/** Original publication reference binds controls; counter values grant nothing. */
internal interface ConnectionMediaChoice { val lease: ConnectionResourceLease }

/** Resource-specific command adapter; generic claim/relay ownership depends only
 * on this focused contract. Adapters hold no mutable latest worker or OS handle. */
internal interface ConnectionMediaConsumerCommands {
    val resource: ConnectionManagement.Resource
    fun stop(lease: ConnectionResourceLease): ConnectionWorkerCommand
    fun retired(lease: ConnectionResourceLease): ConnectionWorkerCommand
}
