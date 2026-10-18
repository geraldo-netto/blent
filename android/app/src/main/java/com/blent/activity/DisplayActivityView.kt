package com.blent.activity

/** Nonblocking read-only relevance. A retained view carries no authentication,
 * desired work, native permission or retirement proof. */
internal interface DisplayActivityView {
    val update: DisplayActivityUpdate
    fun current(snapshot: DisplayActivityUpdate): Boolean
}
