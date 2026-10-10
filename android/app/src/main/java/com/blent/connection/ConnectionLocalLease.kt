package com.blent.connection

/** Shared local metadata shape validation. Values carry no authority; native
 * ownership and current consent are independently checked by boundary adapters. */
internal object ConnectionLocalLease {
    fun validate(lease: ConnectionResourceLease, resource: ConnectionManagement.Resource) {
        require(lease.key.resource == resource)
        require(lease.owner > 0 && lease.connection.epoch > 0)
        require(lease.key.request > 0 && lease.key.generation > 0)
    }
}
