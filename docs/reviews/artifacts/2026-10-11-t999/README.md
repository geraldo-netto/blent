# T999: Linux direct-USB microphone PCM and controls

The typed original USB audio adapter now supports microphone and speakers.
Direction projects to the independently defined resource/profile contract; native
FD registration, invitation, helper ownership and actual cleanup stay shared.
ADB defaults and the complete private ADB pipeline remain. No USB fallback to ADB.
Full profile/peer/credential validation precedes native setup in both directions.

The new permanent accessory capability regression fails before implementation and
passes unchanged afterward. Existing capability assertions now require USB only
when the accessory backend is compiled; unavailable-backend tests still reject
before construction through explicit injected capability. Original ADB, malformed
input, speaker, Stop and ownership assertions are retained. GUI microphone controls
save the exact full peer and settings passively, then Start/Stop only that direction.

Shared private fixtures join real mutual TLS, original broker FD registration,
management/mux, local PCM and synthetic native helpers in both directions. Exact
mono sample and optional clock fields reach the helper; negotiated processing,
buffer/background and authenticated generation are preserved. Cases also cover
pending Stop/source loss before any helper, failed/stalled native setup, original
streaming loss, held actual native dependent leases and delayed remote consumer
proof. Completion remains owned after bounded errors; T995 admission applies to
both directions. No system default or unrelated route is changed.

All builds use cached offline dependencies and exclusively owned private fixtures.
No downloads, installations, live USB/ADB, host services or tablet changes. Native
helper hardware/provider/acoustic qualification remains T835/T859; simultaneous
automated isolation T1000 and physical duplex acceptance remain separate. The
collector's guarded preliminary clean warning for a fresh target is recorded as
T1001; fresh exclusive paths, successful collection and immutable-source/object
attestation still establish these native counters. Protection was never bypassed.

All 55 production functions in host audio/process/controller and GUI audio scopes
pass individual native 80% executable-line coverage, with no missing counters.
Changed authored Rust complexity: 225 functions, maximum 9. Fresh accessory:
45 host + 49 shared tests; default: 41 host + 49 shared. GUI: 111 unit tests and one
private offline Xvfb/D-Bus startup smoke. Full results, compressed LCOV, matching
inventories/object attestations, red/green logs and source fingerprints retained.

All collections include the pre-existing uncommitted TLS fixture; this commit
leaves those unrelated edits unstaged and does not modify them.
