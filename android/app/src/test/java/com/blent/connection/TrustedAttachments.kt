package com.blent.connection

/** Domain tests supply independently trusted events; Activity tests exercise the native boundary. */
internal object TrustedAttachments : AttachmentAdmission {
    override suspend fun take() = true
    override fun consume() {}
}
