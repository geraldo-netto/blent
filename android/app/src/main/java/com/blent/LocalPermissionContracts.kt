package com.blent

/** A permission action retains only portable original-resource state. Implementations
 * must not capture an Activity; its SDK launcher is held only while attached. */
internal interface LocalPermissionAction {
    fun current(): Boolean
    fun complete(granted: Boolean)
}
internal class LocalPermissionAttachment
internal class LocalPermissionPrompt(
    val attachment: LocalPermissionAttachment,
    val action: LocalPermissionAction,
)
