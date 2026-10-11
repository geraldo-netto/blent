# T963: thread-safe Binder fixture ownership

Permanent bounded eight-actor registration/removal regression fails with the
original ArrayList on both SDK configurations: 512 accepted registrations retain
only 434/429 recipients. Every recipient is a unique original identity; every
round requires exact size, all identities, successful original removal, empty
retired list and exact unlink count. Fixture threads must actually terminate.

CopyOnWriteArrayList provides coherent immutable callback snapshots while native
IO and UI observation proceed concurrently. The unlink diagnostic uses an atomic
counter, preserving counts under the same concurrent contract. Eight 512-recipient
rounds pass after the fix. Existing T809/T962 native-death/status/unlink assertions
remain unchanged; remote adapter, native-worker and lane retirement suites pass.
Reviewed named functions stay <=9; T965 tracks omitted Kotlin method kinds.
Production behavior/counters unchanged. Full audio suite race remains separately
under T964. No physical claims or downloads.
