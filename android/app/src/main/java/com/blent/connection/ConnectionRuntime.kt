package com.blent.connection

/** Prompt, pure construction boundary. A throwing factory must retain its own
 * partial native ownership: the supervisor quarantines rather than retrying. */
internal fun interface ConnectionRunFactory {
    fun create(ready: ConnectionSetupReady): ConnectionRun
}

/** One serialized owner, driven by conflated wakeups AND an independent timer.
 * Owns setup plus one runner; local endpoint/process retirement stays external.
 * The stable endpoint facade denies new work after approval loss while allowing
 * exact outstanding consumed/nativeRetired cleanup and terminal events. */
internal interface ConnectionRuntime {
    fun advance()
    fun command(command: ConnectionWorkerCommand): Boolean
    fun poll(): ConnectionWorkerEvent?
    /** Current locally admitted display; null revokes visibility, not ownership. */
    fun display(): LocalDisplayRoute?
    fun endpoints(): ConnectionEndpoints
    fun stop()
    fun failure(): Exception?
    fun retired(): Boolean
    fun terminationRequired(): Boolean
}
