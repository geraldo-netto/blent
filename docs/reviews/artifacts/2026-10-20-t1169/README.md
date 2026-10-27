# T1169 — Android saved-computer settings

Android settings exposes saved computers with explicit refresh and full-fingerprint confirmation. Joined UI/store/Binder tests passed after stale-view cutoff fixes; hosted acceptance does not qualify actual tablet providers.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

ConnectionOwner now composes one cached SavedTrustBinding with its existing
ConnectionAccess and a native-inert AndroidSavedTrustRun. Only application context
is captured; metadata travels through the existing typed START_TRUST operation.
The existing private Binder owner retains Ended/STOP/EXIT/death and original IPC
join semantics. No USB/pairing/media factory is invoked by maintenance.

Settings exposes Saved computers, explicit refresh and full-fingerprint-confirmed
forget. Only a complete retired result is displayed; active or retiring connection
and maintenance access disable conflicting controls. Current-peer revocation,
capacity settings and independent media consent remain separate. A stop may race
an already committed mutation, and the UI says to refresh after cleanup.

DisposableEffect owns one UI command ticket. Disposal revokes it without dropping
accepted work; replacement gets a fresh identity. Confirmation state is scoped to
its view and displayed request. No Activity/view callback enters the application
owner, and no operation is automatically replayed on Activity replacement. The
bounded list renders through a height-constrained LazyColumn with fingerprint
keys. Dialog state and peers are heap-only; native lifetime remains in the
original application lane and private worker.

The access gate exposed an existing fixture mismatch: its fake worker's execute
never returned after the test explicitly released it, so Connect stayed correctly
disabled after denial. The fixture now observes that original release during
execution as well as retirement. Production cutoff/retirement checks and the
permanent pairing assertions were preserved. The initial failure and original
complete pairing input seal are retained; this is fixture alignment, not a native
behavioral fix or permission change.

Permanent Compose tests cover complete/empty results, exact fingerprint
confirmation and dismissal, both admission directions, stop/storage error,
unknown-close quarantine, view loss and late stale commands. Two real Activity
replacement traces retain an accepted forget without duplication and revoke an
unconfirmed selection even for an identical fingerprint. Fixtures settle and join
original work before restoring process owners. Unknown-cleanup simulation uses
an isolated metadata-only access domain, retains its permit through scope loss
and does not leave a fabricated obligation in the shared application fixture.

A UI/store integration uses the existing SavedTrustWorker, exclusive native
filesystem adapter and codec: it confirms one peer, observes successful original
close, reopens the store independently, verifies the new revision and preserved
other peer, then refreshes the UI from persistent storage. Binder tests separately
exercise the production Android adapter, application-context retention boundary,
typed operation, cached application owner and connection exclusion. Hosted Binder
death is explicitly supplied by its test endpoint; it is not evidence of physical
process/device acceptance. Prior native lock/identity regressions remain intact.

The browser now owns its command ticket, with a fresh reference key on each
opening. Logical close revokes that ticket immediately; disposal settles its
UI reference without cancelling accepted work. Confirmation/dismissal callbacks
check the exact current selection before changing it, and an old browser close
cannot revoke its replacement. A read-only client-current observation disables
expired controls; domain actions still revalidate the authoritative owner. New
permanent captured-semantic-action regressions failed before the cutoff fixes,
then passed unchanged. The browser-owner regression separately failed because
Stop remained enabled after ownership loss; its exact oracle also passes after
the guard. Initial read runs only from IDLE on opening, with no replay of an
already accepted operation. Other Activity windows can reopen an expired browser.

One intermediate source attestation was rejected after new regression sources
were added during its test collection. That profile is not qualification. Final
acceptance freezes production, test and tooling source inventory through native
collection and source-hash validation; no counters or functions are excluded.

Final frozen collection: 1403 outer JUnit cases, zero failures, errors
or skips, with verified nested original oracles. JaCoCo 0.8.15 passes all 139
scoped production functions individually at least 80%, including every mapped
native variant and the read-only client-current getter. All production/test/tool
source hashes match the pre-collection manifest. Cached Kotlin 1.9.20 PSI
measures 389 maintained/new production/test entries, maximum 9; native SonarQube
remains T1019. Four architecture checks, offline lint (zero errors, nine existing
warnings) and APK assembly pass. Physical provider/device acceptance remains
T863/T855/T858/T846. No network transfer, installation, live device or push.
Raw evidence: `/backups/disk2/blent-work/2026-10-20-open-items/t1169`; APK:
`/backups/disk2/blent-work/2026-10-20-open-items/t1171/scoped-build/app/outputs/apk/debug/app-debug.apk`.

</details>
